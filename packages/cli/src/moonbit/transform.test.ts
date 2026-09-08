import { afterAll, beforeAll, expect, test } from "bun:test";
import type { Parser } from "web-tree-sitter";
import { createMoonbitParser, parseMoonbit } from "./parser.ts";
import { originalPosition, transformMoonbit, uiAliases } from "./transform.ts";

let parser: Parser;
beforeAll(async () => {
  parser = await createMoonbitParser();
});
afterAll(() => parser.delete());
const transform = (body: string) => {
  const output = transformMoonbit(parser, `fn view() -> @ui.Element {\n${body}\n}`, "view.mbt");
  const tree = parseMoonbit(parser, output.code, "generated.mbt");
  tree.delete();
  return output;
};

test("text, button content, fluent properties, and source nodes bind independently", () => {
  const output = transform(`@ui.div([
    @ui.text("Count: \\{count.get()}"),
    @ui.button("Save \\{count.get()}").disabled(busy.get()).on_click(() => save(count.get())),
    @ui.input().value(name.get()).placeholder(placeholder()),
    @ui.checkbox([]).checked(checked.get()).on_checked_change(checked.set),
    @ui.svg(icon.get()), @ui.image(path.get()), @ui.markdown(read_markdown()),
  ])`);
  expect(output.code).toContain('@ui.div([]).bind_content(() => { "Count: \\{count.get()}" })');
  expect(output.code).toContain('.bind_content(() => { "Save \\{count.get()}" })');
  expect(output.code).toContain(".on_click(() => save(count.get()))");
  expect(output.code).toContain(".on_checked_change(checked.set)");
  expect(output.code.match(/\.bind_value\(/g)).toHaveLength(3);
  expect(output.bindings).toBe(9);
});

test("styles merge in declaration order even across handlers and later static overrides", () => {
  const output = transform(`@ui.div([]).style(@ui.style().bg(theme.get()))
    .width(width.get()).on_click(() => save()).px(12).width(100)
    .hover(s => s.opacity(hover_opacity.get()))`);
  expect(output.code).toContain(
    ".bind_style(() => @ui.style().merge(@ui.style().bg(theme.get())).width(width.get()).px(12).width(100))",
  );
  expect(output.code).toContain(".bind_interaction(");
  expect(output.bindings).toBe(2);
});

test("styles on each side of a slot stay bound to their own node", () => {
  const output = transform("@ui.tabs([]).width(width()).slot(@ui.Panel).height(height())");
  expect(output.code).toContain(
    ".bind_style(() => @ui.style().width(width())).slot(@ui.Panel).bind_style(() => @ui.style().height(height()))",
  );
  expect(output.bindings).toBe(2);
});

test("static literals, setup snapshots, event factories, and explicit bindings stay static", () => {
  const output = transform(`let initial = count.get()
    @ui.div([@ui.text("Count: \\{initial}"), @ui.div([]).bind_content(() => count.get().to_string())])
      .bg(@ui.rgb8(1, 2, 3)).width(100)
      .bind_style(() => @ui.style().opacity(count.get()))
      .hover(s => s.bg(@ui.rgb8(3, 4, 5)))
      .on_click(make_handler(count.get()))`);
  expect(output.bindings).toBe(0);
  expect(output.code.match(/bind_content/g)).toHaveLength(1);
  expect(output.code.match(/bind_style/g)).toHaveLength(1);
  expect(output.code).toContain("let initial = count.get()");
});

test("conditional children use branch selectors and keep sibling positions", () => {
  const output = transform(`@ui.div([
    @ui.text("before"),
    if count.get() > 10 { @ui.text("high") } else if count.get() > 0 { @ui.text("low") } else { @ui.text("zero") },
    @ui.text("after"),
  ])`);
  expect(output.code).toContain(
    ".children_switch(() => { if count.get() > 10 { 0 } else { if count.get() > 0 { 1 } else { 2 } } }",
  );
  expect(output.code.indexOf('.child(@ui.text("before"))')).toBeLessThan(
    output.code.indexOf(".children_switch"),
  );
  expect(output.code.indexOf('.child(@ui.text("after"))')).toBeGreaterThan(
    output.code.indexOf(".children_switch"),
  );
  expect(output.bindings).toBe(1);
});

test("fluent children also compile their nested views", () => {
  const output = transform(
    "@ui.div([]).children([@ui.text(label())]).child(@ui.input().value(value.get()))",
  );
  expect(output.bindings).toBe(2);
  expect(output.code).toContain("bind_content");
});

test("aliases come from the real package import, not the spelling ui", () => {
  expect(uiAliases(parser, 'import { "egoist/quickgui/ui" @gui, "other/ui" }')).toEqual(["gui"]);
  expect(
    uiAliases(
      parser,
      JSON.stringify({ import: [{ path: "egoist/quickgui/ui", alias: "gui" }] }),
      "moon.pkg.json",
    ),
  ).toEqual(["gui"]);
  const source = 'fn view() { @gui.text("\\{count.get()}")\n @ui.text("\\{count.get()}") }';
  const output = transformMoonbit(parser, source, "view.mbt", ["gui"]);
  expect(output.code).toContain("@gui.div([]).bind_content(");
  expect(output.code).toContain('@ui.text("\\{count.get()}")');
});

test("element variable bindings respect parameter shadowing and event-time mutations", () => {
  const output = transform(`let node = @ui.input()
    ignore(node.value(name.get()))
    @ui.button("Save").on_click(() => { ignore(node.value(name.get())) })`);
  expect(output.bindings).toBe(1);
  expect(output.code).toContain(".on_click(() => { ignore(node.value(name.get())) })");
});

test("generated diagnostics map unicode and moved style expressions to original locations", () => {
  const source =
    'fn view() {\n  @ui.text("你好 \\{count.get()}")\n    .width(bad_width())\n    .height(20)\n}';
  const output = transformMoonbit(parser, source, "view.mbt");
  const offset = output.code.indexOf("bad_width");
  const prefix = output.code.slice(0, offset).split("\n");
  expect(originalPosition(output.map, prefix.length, prefix.at(-1)!.length + 1)).toEqual({
    line: 3,
    column: 12,
  });
});

test("invalid syntax fails explicitly rather than leaving a nonreactive snapshot", () => {
  expect(() => transform('@ui.text("\\{count.get()}"')).toThrow("Cannot parse MoonBit");
});

test("view constructor props and handlers are rejected with a fluent migration hint", () => {
  expect(() => transform('@ui.button("Save", on_click=save)')).toThrow("Use .on_click(...)");
  expect(() => transform("@ui.div([], style=@ui.style())")).toThrow("Use .style(...)");
  expect(() => transform("@ui.input(value=name())")).toThrow("Use .value(...)");
});

test("multiline interpolation and raw strings retain their line boundaries", () => {
  const result = transform(
    "@ui.div([@ui.text(\n$| Count: \\{count.get()}\n), @ui.text(\n#| literal \\{value}\n)])",
  );
  expect(result.bindings).toBe(1);
  expect(result.code).toContain("$| Count: \\{count.get()}\n");
  expect(result.code).toContain("#| literal \\{value}\n");
});

test("spread children and array-returning helpers bind collections without misidentifying elements", () => {
  const source =
    'fn rows() -> Array[@ui.Element] { [] }\nfn view() { @ui.div([@ui.text("before"), ..items.get().map(item => @ui.text(item)), @ui.text("after")])\n @ui.div(rows()) }';
  const output = transformMoonbit(parser, source, "view.mbt");
  parseMoonbit(parser, output.code, "generated.mbt").delete();
  expect(output.code).toContain(".bind_content(() => { items.get().map(");
  expect(output.code).toContain(".bind_content(() => { rows() })");
});

test("plain component parameters bind through callers without annotations", () => {
  const source = `fn line_item(product : Product, quantity : Int) -> @ui.Element {
    let initial = quantity
    @ui.text("你好 \\{product.name}: \\{product.price * quantity} / \\{initial}")
  }
  fn cart() -> @ui.Element { line_item({ name: "Mug", price: 12 }, quantity()) }`;
  const output = transformMoonbit(parser, source, "cart.mbt");
  parseMoonbit(parser, output.code, "generated.mbt").delete();
  expect(output.code).toContain("product : () -> Product, quantity : () -> Int");
  expect(output.code).toContain("let initial = quantity()");
  expect(output.code).toContain("product().price * quantity()");
  expect(output.code).toContain('quickgui_component_line_item_value({ name: "Mug", price: 12 })');
  const at = output.code.indexOf("product().price");
  const prefix = output.code.slice(0, at).split("\n");
  const expected = source.slice(0, source.indexOf("product.price")).split("\n");
  expect(originalPosition(output.map, prefix.length, prefix.at(-1)!.length + 1)).toEqual({
    line: expected.length,
    column: expected.at(-1)!.length + 1,
  });
});

test("component prop reads respect local, callback, match, and loop shadows", () => {
  const source = `fn label(value : String) -> @ui.Element {
    let echo = value => value
    for value in ["one"] { ignore(value) }
    let selected = match Some("two") { Some(value) => value; None => value }
    let value = echo(value)
    @ui.text(value + selected)
  }`;
  const output = transformMoonbit(parser, source, "shadow.mbt");
  parseMoonbit(parser, output.code, "generated.mbt").delete();
  expect(output.code).toContain("let echo = value => value");
  expect(output.code).toContain('for value in ["one"] { ignore(value) }');
  expect(output.code).toContain("Some(value) => value; None => value()");
  expect(output.code).toContain("let value = echo(value())");
  expect(output.code).toContain("@ui.text(value + selected)");
});
