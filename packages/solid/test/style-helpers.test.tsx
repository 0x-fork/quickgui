import { expect, test } from "bun:test";
import { createSignal, flush } from "solid-js";
import { app, parseColor, PropertyCode, Window } from "@quickgui/native";
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
} from "../src/index.ts";

await app.whenReady();

test("JSX style objects expose camelCase presets on primitives and compound parts", () => {
  const host = new Window({
    renderer: createRenderer(() => (
      <View
        style={{
          flexCol: true,
          itemsCenter: true,
          p3: true,
          gap2: true,
          roundedLg: true,
          width: 320,
          height: 120,
          bg: "#090d16",
          textColor: "#e2e8f0",
        }}
      >
        <Text style={{ textLg: true, fontSemibold: true, textCenter: true, cursorPointer: true }}>
          Title
        </Text>
        <Tabs.Root value="one">
          <Tabs.List style={{ roundedMd: true, px3: true, bg: "#18181b" }} />
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
  expect(node.properties.get(PropertyCode.BackgroundColor)).toBe(parseColor("#090d16"));
  expect(node.properties.get(PropertyCode.Color)).toBe(parseColor("#e2e8f0"));
  const label = node.children[0]!;
  expect(label.properties.get(PropertyCode.FontSize)).toBe(18);
  expect(label.properties.get(PropertyCode.LineHeight)).toBe(26);
  expect(label.properties.get(PropertyCode.FontWeight)).toBe(600);
  expect(label.properties.get(PropertyCode.TextAlign)).toBe("center");
  expect(label.properties.get(PropertyCode.Cursor)).toBe("pointer");
  const list = node.children[1]!.children[0]!;
  expect(list.properties.get(PropertyCode.BorderRadius)).toBe(6);
  expect(list.properties.get(PropertyCode.PaddingLeft)).toBe(12);
  expect(list.properties.get(PropertyCode.BackgroundColor)).toBe(parseColor("#18181b"));
  host.close();
});

test("camelCase style values project colors, gradients, images, and transitions", () => {
  const [color, setColor] = createSignal("#090d16");
  const gradient = "linear-gradient(90deg, #1d4ed8, #38bdf8)";
  const imageStyle = {
    bgImage: "/assets/paper.png",
    bgSize: "cover",
    bgRepeat: "no-repeat",
    bgPosition: "center",
    hoverBg: "#1d4ed8",
    activeBg: "#0b1220",
    focusBg: gradient,
  } satisfies JSX.Style;
  const host = new Window({
    renderer: createRenderer(() => (
      <View style={[imageStyle, { bg: color(), transitionProperty: "bg, opacity" }]}>
        <View style={{ bgGradient: gradient }} />
      </View>
    )),
  });
  const node = host.root.children[0]!,
    child = node.children[0]!;
  expect(node.properties.get(PropertyCode.BackgroundColor)).toBe(parseColor("#090d16"));
  expect(node.properties.get(PropertyCode.BackgroundImage)).toBe("/assets/paper.png");
  expect(node.properties.get(PropertyCode.BackgroundSize)).toBe("cover");
  expect(node.properties.get(PropertyCode.BackgroundRepeat)).toBe("no-repeat");
  expect(node.properties.get(PropertyCode.BackgroundPosition)).toBe("center");
  expect(node.properties.get(PropertyCode.HoverBackgroundColor)).toBe(parseColor("#1d4ed8"));
  expect(node.properties.get(PropertyCode.ActiveBackgroundColor)).toBe(parseColor("#0b1220"));
  expect(node.properties.get(PropertyCode.FocusBackgroundGradient)).toBe(gradient);
  expect(node.properties.get(PropertyCode.TransitionProperties)).toBe("background-color,opacity");
  expect(child.properties.get(PropertyCode.BackgroundGradient)).toBe(gradient);
  setColor("#ffffff");
  flush();
  expect(host.root.children[0]).toBe(node);
  expect(node.children[0]).toBe(child);
  expect(node.properties.get(PropertyCode.BackgroundColor)).toBe(parseColor("#ffffff"));
  setProp(child, "style", undefined);
  expect(child.properties.has(PropertyCode.BackgroundGradient)).toBe(false);
  host.close();
});

test("presets use boolean style entries and custom radii use camelCase values", () => {
  for (const [helper, expected] of Object.entries({
    roundedSm: 4,
    roundedMd: 6,
    roundedLg: 8,
    roundedXl: 12,
    rounded2xl: 16,
    roundedFull: 4096,
  })) {
    const node = createComponent(View, { style: { [helper]: true } });
    expect(node.properties.get(PropertyCode.BorderRadius)).toBe(expected);
  }
  const node = createComponent(View, {
    style: {
      borderRadius: 10,
      borderTopLeftRadius: 8,
      borderTopRightRadius: 8,
      borderBottomRightRadius: 3,
    },
  });
  expect(node.properties.get(PropertyCode.BorderRadius)).toBe(10);
  expect(node.properties.get(PropertyCode.BorderTopLeftRadius)).toBe(8);
  expect(node.properties.get(PropertyCode.BorderTopRightRadius)).toBe(8);
  expect(node.properties.get(PropertyCode.BorderBottomRightRadius)).toBe(3);
  expect(() => setProp(node, "style", { roundedLg: "lg" })).toThrow("boolean");
  expect(() => setProp(node, "style", { p3: 3 })).toThrow("boolean");
  expect(() => setProp(node, "style", { fontSize: 20, "text-lg": true })).toThrow("camelCase");
  expect(node.properties.has(PropertyCode.FontSize)).toBe(false);
  expect(() => setProp(node, "text-lg", true)).toThrow("style");
  expect(() => setProp(node, "textLg", true)).toThrow("style");
  expect(() => setProp(node, "flex", true)).toThrow("style");
  expect(() => setProp(node, "fontSize", 18)).toThrow("style");
  expect(() => setProp(node, "font-size", 18)).toThrow("style");

  // @ts-expect-error Presets belong in the style object.
  const directive: JSX.NativeProps = { textLg: true };
  // @ts-expect-error Native styling belongs in the style object.
  const direct: JSX.NativeProps = { fontSize: 18 };
  // @ts-expect-error Style keys are camelCase.
  const kebab: JSX.Style = { "text-lg": true };
  // @ts-expect-error Presets are boolean, not size values.
  const invalid: JSX.Style = { roundedLg: 8 };
  void directive;
  void direct;
  void kebab;
  void invalid;
});

test("style arrays expand presets in declaration order and restore earlier values", () => {
  const panel = { roundedLg: true, p3: true, textLg: true } satisfies JSX.Style;
  const base = flattenStyle([panel, false, [{ roundedXl: true, paddingLeft: 20 }]]);
  expect(base.borderRadius).toBe(12);
  expect(base.paddingLeft).toBe(20);
  expect(base.paddingRight).toBe(12);
  expect(base.fontSize).toBe(18);
  expect(base.lineHeight).toBe(26);
  const node = createElement("view");
  setProp(node, "style", [base, { roundedSm: true }]);
  expect(node.properties.get(PropertyCode.BorderRadius)).toBe(4);
  const changed = { ...base, rounded2xl: true } satisfies JSX.Style;
  setProp(node, "style", [changed, { roundedSm: true }]);
  expect(node.properties.get(PropertyCode.BorderRadius)).toBe(4);
  setProp(node, "style", [changed, false]);
  expect(node.properties.get(PropertyCode.BorderRadius)).toBe(16);
  setProp(node, "style", undefined);
  expect(node.properties.size).toBe(0);
});

test("reactive style presets retain nodes and restore all affected fields", async () => {
  const [enabled, setEnabled] = createSignal(true),
    [rounded, setRounded] = createSignal(true),
    [large, setLarge] = createSignal(false),
    [corner, setCorner] = createSignal<number | undefined>(3);
  const host = new Window({
    renderer: createRenderer(() => (
      <View
        style={[
          {
            borderRadius: 2,
            borderTopLeftRadius: 20,
            fontSize: 14,
            lineHeight: 20,
            paddingLeft: 8,
          },
          { textLg: enabled(), p3: enabled(), roundedLg: rounded(), roundedXl: large() },
          corner() === undefined ? undefined : { borderTopLeftRadius: corner()! },
        ]}
      >
        <Text>Retained</Text>
      </View>
    )),
  });
  const node = host.root.children[0]!,
    child = node.children[0]!;
  expect(node.properties.get(PropertyCode.BorderRadius)).toBe(8);
  expect(node.properties.get(PropertyCode.BorderTopLeftRadius)).toBe(3);
  expect(node.properties.get(PropertyCode.PaddingLeft)).toBe(12);
  setLarge(true);
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
  setCorner(undefined);
  flush();
  expect(node.properties.has(PropertyCode.BorderTopLeftRadius)).toBe(false);
  setRounded(false);
  setLarge(false);
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

test("equivalent styles and hidden fallback changes submit no native work", async () => {
  const host = new Window({ renderer: createRenderer(() => <View style={{ roundedLg: true }} />) });
  await host.whenReady();
  await Promise.resolve();
  const node = host.root.children[0]!,
    before = callsNamed("applyBatch").length;
  setProp(node, "style", { roundedLg: true });
  setProp(node, "style", [{ roundedXl: true }, { roundedLg: true }]);
  await Promise.resolve();
  expect(callsNamed("applyBatch")).toHaveLength(before);
  setProp(node, "style", [{ roundedXl: true }, false]);
  await Promise.resolve();
  expect(node.properties.get(PropertyCode.BorderRadius)).toBe(12);
  expect(callsNamed("applyBatch")).toHaveLength(before + 1);
  host.close();
});

test("layout presets compose with native values in one style object", () => {
  const layout = {
    grid: true,
    gridTemplateColumns: 3,
    gridTemplateRows: "repeat(2, minmax(min-content, 1fr))",
    gridColumnSpan: 2,
    rowSpanFull: true,
    width: "50%",
    hFull: true,
    rtl: true,
    paddingStart: 12,
    borderEndWidth: 1,
    positionSticky: true,
    top: 8,
    overflowXScroll: true,
    scrollSnapAlign: "center",
  } satisfies JSX.Style;
  const node = createComponent(View, { style: layout });
  expect(node.properties.get(PropertyCode.GridTemplateColumns)).toBe(3);
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
  setProp(node, "style", [layout, { colSpanFull: true }]);
  expect(node.properties.has(PropertyCode.GridColumnSpan)).toBe(false);
  expect(node.properties.get(PropertyCode.GridColumnEnd)).toBe(-1);
  setProp(node, "style", [layout, false]);
  expect(node.properties.get(PropertyCode.GridColumnSpan)).toBe(2);
  expect(node.properties.has(PropertyCode.GridColumnEnd)).toBe(false);
});

test("flex presets and CSS flex shorthand compose inside style", () => {
  const node = createComponent(View, { style: { flex: true, flexWrap: true } });
  expect(node.properties.get(PropertyCode.Display)).toBe("flex");
  expect(node.properties.get(PropertyCode.FlexWrap)).toBe("wrap");
  setProp(node, "style", [
    { flex: true, flexWrap: true },
    { flex: "2 0 20px", flexWrap: "wrap-reverse" },
  ]);
  expect(node.properties.get(PropertyCode.FlexGrow)).toBe(2);
  expect(node.properties.get(PropertyCode.FlexShrink)).toBe(0);
  expect(node.properties.get(PropertyCode.FlexBasis)).toBe(20);
  expect(node.properties.get(PropertyCode.FlexWrap)).toBe("wrap-reverse");
  setProp(node, "style", [{ flex: "2 0 20px", flexWrap: "wrap-reverse" }, { flexWrap: true }]);
  expect(node.properties.get(PropertyCode.FlexWrap)).toBe("wrap");
  setProp(node, "style", undefined);
  expect(node.properties.has(PropertyCode.FlexGrow)).toBe(false);
  expect(node.properties.has(PropertyCode.FlexShrink)).toBe(false);
  expect(node.properties.has(PropertyCode.FlexBasis)).toBe(false);
});

test("grid shorthands and preset spans restore one another across reactive updates", () => {
  const [full, setFull] = createSignal(true);
  const host = new Window({
    renderer: createRenderer(() => (
      <div style={[{ gridColumn: "1 / 4" }, { colSpanFull: full(), roundedLg: true }]} />
    )),
  });
  const node = host.root.children[0]!;
  expect(node.properties.get(PropertyCode.BorderRadius)).toBe(8);
  expect(node.properties.get(PropertyCode.GridColumnStart)).toBe(1);
  expect(node.properties.get(PropertyCode.GridColumnEnd)).toBe(-1);
  setFull(false);
  flush();
  expect(node.properties.get(PropertyCode.GridColumnStart)).toBe(1);
  expect(node.properties.get(PropertyCode.GridColumnEnd)).toBe(4);
  expect(node.properties.has(PropertyCode.GridColumnSpan)).toBe(false);
  setProp(node, "style", [{ gridColumnSpan: 2 }, { gridColumn: "2 / span 3" }]);
  expect(node.properties.get(PropertyCode.GridColumnStart)).toBe(2);
  expect(node.properties.get(PropertyCode.GridColumnEnd)).toBe(5);
  expect(node.properties.has(PropertyCode.GridColumnSpan)).toBe(false);
  setProp(node, "style", undefined);
  expect(node.properties.has(PropertyCode.GridColumnStart)).toBe(false);
  expect(node.properties.has(PropertyCode.GridColumnEnd)).toBe(false);
  host.close();
});

test("span and Text share style objects while behavior stays in ordinary props", () => {
  const host = new Window({
    renderer: createRenderer(() => (
      <div>
        <Text style={{ fontSize: 18, fontWeight: 600 }}>some text</Text>
        <span aria-label="caption" style={{ fontSize: 18, fontWeight: 600 }}>
          some text
        </span>
      </div>
    )),
  });
  const [label, span] = host.root.children[0]!.children;
  expect(span!.tag).toBe(label!.tag);
  expect(span!.properties.get(PropertyCode.FontSize)).toBe(18);
  expect(span!.properties.get(PropertyCode.FontWeight)).toBe(600);
  expect(span!.properties.get(PropertyCode.AccessibilityLabel)).toBe("caption");
  host.close();
  // @ts-expect-error Styling uses style objects.
  const classProp: JSX.NativeProps = { class: "text-lg" };
  // @ts-expect-error className is not a native JSX prop.
  const classNameProp: JSX.NativeProps = { className: "text-lg" };
  void classProp;
  void classNameProp;
});
