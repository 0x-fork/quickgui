import { afterAll, beforeAll, expect, test } from "bun:test";
import type { Parser } from "web-tree-sitter";
import { childLists } from "../../../../scripts/moonbit-child-lists.ts";
import { signalPairs } from "../../../../scripts/moonbit-signal-pairs.ts";
import { fluentProps } from "../../../../scripts/moonbit-fluent-props.ts";
import { createMoonbitParser, parseMoonbit } from "./parser.ts";

let parser: Parser;
beforeAll(async () => {
  parser = await createMoonbitParser();
});
afterAll(() => parser.delete());

test("constructor styles and events migrate to fluent methods after children", () => {
  const result = fluentProps(
    parser,
    `fn view() {
    @ui.view([@ui.input(value=name(), on_input=set_name)], style=@ui.style().padding(24), on_click=save)
  }`,
    "view.mbt",
  );
  expect(result).toContain(
    "@ui.view([@ui.input().value(name()).on_input(set_name)]).style(@ui.style().padding(24)).on_click(save)",
  );
  parseMoonbit(parser, result, "generated.mbt").delete();
});

test("child-list examples attach children to the created slot and preserve label order", () => {
  const result = childLists(
    parser,
    `fn view() {
    let tabs = @ui.tabs([])
    tabs.child(tabs.slot(@ui.Panel).label("Heading").child(@ui.text("Content")))
  }`,
    "view.mbt",
  );
  expect(result).toContain('tabs.slot(@ui.Panel).label("Heading").children([@ui.text("Content")])');
  expect(result).not.toContain("tabs.children([@ui.text");
  parseMoonbit(parser, result, "generated.mbt").delete();
});

test("signal-pair examples retain the type of ambiguous initial values", () => {
  const result = signalPairs(
    parser,
    `fn view() {
    let selected : @reactive.Signal[Json] = @reactive.signal([])
    selected.set(Json::null())
    selected.get()
  }`,
    "view.mbt",
  );
  expect(result).toContain("@ui.create_signal(([] : Json))");
  expect(result).toContain("set_selected(Json::null())");
  parseMoonbit(parser, result, "generated.mbt").delete();
});

test("signal update blocks keep their return scope and read the previous value once", () => {
  const result = signalPairs(
    parser,
    `fn view() {
    let items = @reactive.signal([1])
    items.update(previous => {
      if previous.is_empty() { return previous }
      previous.copy()
    })
  }`,
    "view.mbt",
  );
  expect(result).toContain("set_items((() => { let previous = @ui.untrack(items)");
  expect(result.match(/@ui\.untrack\(items\)/g)).toHaveLength(1);
  expect(result).toContain("return previous");
  parseMoonbit(parser, result, "generated.mbt").delete();
});
