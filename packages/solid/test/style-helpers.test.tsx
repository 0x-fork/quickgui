import { expect, test } from "bun:test";
import { createSignal, flush } from "solid-js";
import { app, PropertyCode, Window } from "@quickgui/native";
import { callsNamed } from "../../native/test/fake-binding.ts";
import {
  createComponent,
  createElement,
  createRenderer,
  flattenStyle,
  setProp,
  Tabs,
  Text,
  View,
  type JSX,
  type RoundedPreset,
} from "../src/index.ts";

await app.whenReady();

test("compiled JSX exposes kebab-case Rust helpers on primitives and compound parts", () => {
  const host = new Window({
    renderer: createRenderer(() => (
      <View flex-col items-center p-3 gap-2 rounded="lg" size={[320, 120]}>
        <Text text-lg font-semibold text-center cursor-pointer>
          Title
        </Text>
        <Tabs.Root value="one">
          <Tabs.List rounded="md" px={12} />
        </Tabs.Root>
      </View>
    )),
  });
  const node = host.root.children[0]!;
  expect(node.properties.get(PropertyCode.Display)).toBe("flex");
  expect(node.properties.get(PropertyCode.FlexDirection)).toBe("column");
  expect(node.properties.get(PropertyCode.AlignItems)).toBe("center");
  expect(node.properties.get(PropertyCode.PaddingLeft)).toBe(12);
  expect(node.properties.get(PropertyCode.PaddingBottom)).toBe(12);
  expect(node.properties.get(PropertyCode.Gap)).toBe(8);
  expect(node.properties.get(PropertyCode.BorderRadius)).toBe(8);
  expect(node.properties.get(PropertyCode.Width)).toBe(320);
  expect(node.properties.get(PropertyCode.Height)).toBe(120);
  const label = node.children[0]!;
  expect(label.properties.get(PropertyCode.FontSize)).toBe(18);
  expect(label.properties.get(PropertyCode.LineHeight)).toBe(26);
  expect(label.properties.get(PropertyCode.FontWeight)).toBe(600);
  expect(label.properties.get(PropertyCode.TextAlign)).toBe("center");
  expect(label.properties.get(PropertyCode.Cursor)).toBe("pointer");
  const list = node.children[1]!.children[0]!;
  expect(list.properties.get(PropertyCode.BorderRadius)).toBe(6);
  expect(list.properties.get(PropertyCode.PaddingLeft)).toBe(12);
  host.close();
});

test("named radii and numeric corner helpers use the Rust scale", () => {
  const radii: Record<RoundedPreset, number> = {
    sm: 4,
    md: 6,
    lg: 8,
    xl: 12,
    "2xl": 16,
    full: 4096,
  };
  for (const [rounded, expected] of Object.entries(radii)) {
    const node = createComponent(View, { rounded: rounded as RoundedPreset });
    expect(node.properties.get(PropertyCode.BorderRadius)).toBe(expected);
  }
  const node = createComponent(View, { rounded: 10, "rounded-t": "lg", "rounded-br": 3 });
  expect(node.properties.get(PropertyCode.BorderRadius)).toBe(10);
  expect(node.properties.get(PropertyCode.BorderTopLeftRadius)).toBe(8);
  expect(node.properties.get(PropertyCode.BorderTopRightRadius)).toBe(8);
  expect(node.properties.get(PropertyCode.BorderBottomRightRadius)).toBe(3);
  expect(() => setProp(node, "rounded", "huge")).toThrow("finite radius");
  expect(() => setProp(node, "rounded", NaN)).toThrow("finite radius");
  // @ts-expect-error Radius names are checked in JSX as well as at runtime.
  const invalid: JSX.NativeProps = { rounded: "huge" };
  void invalid;
  // @ts-expect-error Scale helpers use kebab-case, including the separator before the number.
  const invalidSpacing: JSX.NativeProps = { p3: true };
  void invalidSpacing;
});

test("style arrays expand helpers in declaration order and direct props restore style fallbacks", () => {
  const panel = { rounded: "lg", "p-3": true, "text-lg": true } satisfies JSX.Style;
  const style = flattenStyle([panel, false, [{ "rounded-xl": true, paddingLeft: 20 }]]);
  expect(style.borderRadius).toBe(12);
  expect(style.paddingLeft).toBe(20);
  expect(style.paddingRight).toBe(12);
  expect(style.fontSize).toBe(18);
  expect(style.lineHeight).toBe(26);
  for (const directFirst of [true, false]) {
    const node = createElement("view");
    if (directFirst) setProp(node, "rounded", "sm");
    setProp(node, "style", style);
    if (!directFirst) setProp(node, "rounded", "sm");
    expect(node.properties.get(PropertyCode.BorderRadius)).toBe(4);
    setProp(node, "style", { ...style, rounded: "2xl" });
    expect(node.properties.get(PropertyCode.BorderRadius)).toBe(4);
    setProp(node, "rounded", undefined);
    expect(node.properties.get(PropertyCode.BorderRadius)).toBe(16);
    setProp(node, "style", undefined);
    expect(node.properties.size).toBe(0);
  }
});

test("reactive helpers retain nodes and restore every affected field without overwriting sibling props", async () => {
  const [enabled, setEnabled] = createSignal(true);
  const [rounded, setRounded] = createSignal<RoundedPreset | undefined>("lg");
  const host = new Window({
    renderer: createRenderer(() => (
      <View
        style={{
          borderRadius: 2,
          borderTopLeftRadius: 20,
          fontSize: 14,
          lineHeight: 20,
          paddingLeft: 8,
        }}
        text-lg={enabled()}
        p-3={enabled()}
        rounded={rounded()}
        borderTopLeftRadius={3}
      >
        <Text>Retained</Text>
      </View>
    )),
  });
  const node = host.root.children[0]!;
  const child = node.children[0]!;
  expect(node.properties.get(PropertyCode.BorderRadius)).toBe(8);
  expect(node.properties.get(PropertyCode.BorderTopLeftRadius)).toBe(3);
  expect(node.properties.get(PropertyCode.PaddingLeft)).toBe(12);
  setRounded("xl");
  setEnabled(false);
  flush();
  expect(host.root.children[0]).toBe(node);
  expect(node.children[0]).toBe(child);
  expect(node.properties.get(PropertyCode.BorderRadius)).toBe(12);
  expect(node.properties.get(PropertyCode.BorderTopLeftRadius)).toBe(3);
  expect(node.properties.get(PropertyCode.FontSize)).toBe(14);
  expect(node.properties.get(PropertyCode.LineHeight)).toBe(20);
  expect(node.properties.get(PropertyCode.PaddingLeft)).toBe(8);
  expect(node.properties.has(PropertyCode.PaddingRight)).toBe(false);
  setProp(node, "borderTopLeftRadius", undefined);
  flush();
  // Rust's uniform rounded helper explicitly resets the per-corner radii.
  expect(node.properties.has(PropertyCode.BorderTopLeftRadius)).toBe(false);
  setRounded(undefined);
  flush();
  expect(node.properties.get(PropertyCode.BorderRadius)).toBe(2);
  expect(node.properties.get(PropertyCode.BorderTopLeftRadius)).toBe(20);
  setEnabled(true);
  flush();
  expect(node.properties.get(PropertyCode.FontSize)).toBe(18);
  expect(node.properties.get(PropertyCode.LineHeight)).toBe(26);
  expect(node.properties.get(PropertyCode.PaddingLeft)).toBe(12);
  host.close();
  await Promise.resolve();
  setEnabled(false);
  flush();
  expect(host.nodes.size).toBe(0);
});

test("equivalent helpers and hidden fallbacks submit no native work", async () => {
  const host = new Window({
    renderer: createRenderer(() => <View style={{ rounded: "lg" }} rounded={8} />),
  });
  await host.whenReady();
  await Promise.resolve();
  const node = host.root.children[0]!;
  const before = callsNamed("applyBatch").length;
  setProp(node, "rounded", "lg");
  setProp(node, "style", { rounded: "xl" });
  await Promise.resolve();
  expect(callsNamed("applyBatch")).toHaveLength(before);
  setProp(node, "rounded", false);
  await Promise.resolve();
  expect(node.properties.get(PropertyCode.BorderRadius)).toBe(12);
  expect(callsNamed("applyBatch")).toHaveLength(before + 1);
  host.close();
});

test("grid, fraction, logical spacing, overflow, and position helpers reach native properties", () => {
  const node = createComponent(View, {
    grid: true,
    "grid-cols": 3,
    "grid-rows-min-content": 2,
    "col-span": 2,
    "row-span-full": true,
    "w-fraction": 0.5,
    "h-full": true,
    rtl: true,
    ps: 12,
    "border-e": 1,
    "position-sticky": true,
    "sticky-top": 8,
    "overflow-x-scroll": true,
    "snap-align": "center",
  });
  expect(node.properties.get(PropertyCode.GridTemplateColumns)).toBe("repeat(3, minmax(0, 1fr))");
  expect(node.properties.get(PropertyCode.GridTemplateRows)).toBe(
    "repeat(2, minmax(min-content, 1fr))",
  );
  expect(node.properties.get(PropertyCode.GridColumnSpan)).toBe(2);
  expect(node.properties.get(PropertyCode.GridRowStart)).toBe(1);
  expect(node.properties.get(PropertyCode.GridRowEnd)).toBe(-1);
  expect(node.properties.get(PropertyCode.Width)).toBe("50%");
  expect(node.properties.get(PropertyCode.Height)).toBe("100%");
  expect(node.properties.get(PropertyCode.Direction)).toBe("rtl");
  expect(node.properties.get(PropertyCode.PaddingStart)).toBe(12);
  expect(node.properties.get(PropertyCode.BorderEndWidth)).toBe(1);
  expect(node.properties.get(PropertyCode.Position)).toBe("sticky");
  expect(node.properties.get(PropertyCode.Top)).toBe(8);
  expect(node.properties.get(PropertyCode.OverflowX)).toBe("scroll");
  expect(node.properties.get(PropertyCode.OverflowY)).toBe("hidden");
  expect(node.properties.get(PropertyCode.ScrollSnapAlign)).toBe("center");
  setProp(node, "grid-cols", 2000);
  expect(node.properties.get(PropertyCode.GridTemplateColumns)).toBe(
    "repeat(1024, minmax(0, 1fr))",
  );
  setProp(node, "grid-cols", 0);
  expect(node.properties.has(PropertyCode.GridTemplateColumns)).toBe(false);
  setProp(node, "w-fraction", -1);
  expect(node.properties.get(PropertyCode.Width)).toBe("0%");
  setProp(node, "col-span-full", true);
  expect(node.properties.has(PropertyCode.GridColumnSpan)).toBe(false);
  expect(node.properties.get(PropertyCode.GridColumnEnd)).toBe(-1);
  setProp(node, "col-span-full", false);
  expect(node.properties.get(PropertyCode.GridColumnSpan)).toBe(2);
  expect(node.properties.has(PropertyCode.GridColumnEnd)).toBe(false);
});

test("boolean flex helpers coexist with CSS flex and flexWrap values", () => {
  const node = createComponent(View, { flex: true, "flex-wrap": true });
  expect(node.properties.get(PropertyCode.Display)).toBe("flex");
  expect(node.properties.get(PropertyCode.FlexWrap)).toBe("wrap");
  setProp(node, "flex", 1);
  expect(node.properties.has(PropertyCode.Display)).toBe(false);
  expect(node.properties.get(PropertyCode.FlexGrow)).toBe(1);
  expect(node.properties.get(PropertyCode.FlexShrink)).toBe(1);
  expect(node.properties.get(PropertyCode.FlexBasis)).toBe(0);
  setProp(node, "flex", "2 0 20px");
  expect(node.properties.get(PropertyCode.FlexGrow)).toBe(2);
  expect(node.properties.get(PropertyCode.FlexShrink)).toBe(0);
  expect(node.properties.get(PropertyCode.FlexBasis)).toBe(20);
  setProp(node, "flex-wrap", "wrap-reverse");
  expect(node.properties.get(PropertyCode.FlexWrap)).toBe("wrap-reverse");
  setProp(node, "flexWrap", "nowrap");
  setProp(node, "flex-wrap", true);
  expect(node.properties.get(PropertyCode.FlexWrap)).toBe("nowrap");
  setProp(node, "flexWrap", undefined);
  expect(node.properties.get(PropertyCode.FlexWrap)).toBe("wrap");
  setProp(node, "flex", false);
  expect(node.properties.has(PropertyCode.FlexGrow)).toBe(false);
  expect(node.properties.has(PropertyCode.FlexShrink)).toBe(false);
  expect(node.properties.has(PropertyCode.FlexBasis)).toBe(false);
});

test("grid shorthands and helper spans restore one another across reactive updates", () => {
  const [span, setSpan] = createSignal<number | undefined>(2);
  const host = new Window({
    renderer: createRenderer(() => (
      <view style={{ gridColumn: "1 / 4" }} col-span={span()} rounded="lg" />
    )),
  });
  const node = host.root.children[0]!;
  expect(node.properties.get(PropertyCode.BorderRadius)).toBe(8);
  expect(node.properties.get(PropertyCode.GridColumnSpan)).toBe(2);
  expect(node.properties.has(PropertyCode.GridColumnStart)).toBe(false);
  setSpan(undefined);
  flush();
  expect(node.properties.get(PropertyCode.GridColumnStart)).toBe(1);
  expect(node.properties.get(PropertyCode.GridColumnEnd)).toBe(4);
  expect(node.properties.has(PropertyCode.GridColumnSpan)).toBe(false);
  setProp(node, "style", [{ "col-span": 2 }, { gridColumn: "2 / span 3" }]);
  expect(node.properties.get(PropertyCode.GridColumnStart)).toBe(2);
  expect(node.properties.get(PropertyCode.GridColumnEnd)).toBe(5);
  expect(node.properties.has(PropertyCode.GridColumnSpan)).toBe(false);
  setProp(node, "style", undefined);
  expect(node.properties.has(PropertyCode.GridColumnStart)).toBe(false);
  expect(node.properties.has(PropertyCode.GridColumnEnd)).toBe(false);
  host.close();
});
