import { app, Window } from "@quickgui/native";
import { Button, Text, View, createRenderer, type NativeNode, type Style } from "@quickgui/ui";
import { For } from "@quickgui/ui";

await app.whenReady();

const ink = "#e6ecf7";
const muted = "#94a3b8";
const panelBackground = "#141a26";
const panelBorder = "#27324a";

const panelStyle: Style = {
  display: "flex",
  flexDirection: "column",
  gap: 12,
  padding: 18,
  backgroundColor: panelBackground,
  borderColor: panelBorder,
  borderWidth: 1,
  borderRadius: 14,
};

const captionStyle: Style = {
  color: muted,
  fontSize: 11,
  letterSpacing: 0.8,
  textTransform: "uppercase",
  fontWeight: 700,
};

function Panel(props: { title: string; children?: NativeNode[] }) {
  return (
    <View style={panelStyle}>
      <Text style={captionStyle}>{props.title}</Text>
      {props.children}
    </View>
  );
}

type TextAlignValue = "left" | "center" | "right" | "justify" | "start" | "end";
const textAlignments: TextAlignValue[] = ["left", "center", "right", "justify", "start", "end"];

/** `textAlign` including the direction-relative `start` and `end` the core resolves itself. */
function TextAlignment() {
  return (
    <Panel title="Text alignment">
      <For each={textAlignments}>
        {(align) => (
          <View
            style={{
              width: "100%",
              padding: 8,
              borderRadius: 8,
              backgroundColor: "#1b2434",
            }}
          >
            <Text style={{ width: "100%", textAlign: align, fontSize: 13, color: ink }}>
              {`textAlign: "${align}" — the core resolves start and end against the inherited direction.`}
            </Text>
          </View>
        )}
      </For>
    </Panel>
  );
}

/** Letter and word spacing, case mapping, shadows, decorations, and break policy. */
function TextStyling() {
  return (
    <Panel title="Extended text styling">
      <Text style={{ fontSize: 20, fontWeight: 700, letterSpacing: 2, color: ink }}>
        letterSpacing 2
      </Text>
      <Text style={{ fontSize: 14, wordSpacing: 8, color: ink }}>
        wordSpacing 8 pushes every space apart
      </Text>
      <Text style={{ fontSize: 14, textTransform: "capitalize", color: ink }}>
        textTransform capitalize keeps selection on the original text
      </Text>
      <Text
        style={{
          fontSize: 24,
          fontWeight: 700,
          color: "#f8fafc",
          textShadow: "0 3px 10px #38bdf8aa",
        }}
      >
        textShadow
      </Text>
      <Text
        style={{
          fontSize: 14,
          color: ink,
          textDecoration: "underline",
          textDecorationColor: "#f97316",
          textDecorationStyle: "wavy",
          textDecorationThickness: 2,
        }}
      >
        wavy underline in its own color
      </Text>
      <Text
        style={{
          fontSize: 14,
          color: ink,
          textDecoration: "line-through overline",
          textDecorationColor: "#f43f5e",
        }}
      >
        line-through and overline together
      </Text>
      <View style={{ width: 200, padding: 8, borderRadius: 8, backgroundColor: "#1b2434" }}>
        <Text
          style={{
            width: "100%",
            fontSize: 13,
            color: muted,
            wordBreak: "break-all",
            overflowWrap: "anywhere",
            hyphens: "manual",
          }}
        >
          wordBreak break-all with overflowWrap anywhere: supercalifragilisticexpialidocious
        </Text>
      </View>
    </Panel>
  );
}

/** `direction: "rtl"` mirrors in-flow positions; the logical edges follow it. */
function Direction() {
  return (
    <Panel title="Right to left">
      <View
        style={{
          direction: "rtl",
          display: "flex",
          flexDirection: "row",
          gap: 8,
          alignItems: "center",
          padding: 10,
          paddingStart: 20,
          borderRadius: 10,
          borderStartWidth: 3,
          borderColor: "#38bdf8",
          backgroundColor: "#1b2434",
        }}
      >
        <View style={{ width: 28, height: 20, borderRadius: 6, backgroundColor: "#38bdf8" }} />
        <View style={{ width: 20, height: 20, borderRadius: 6, backgroundColor: "#334155" }} />
        <Text style={{ fontSize: 13, color: ink, textAlign: "start", textDirection: "rtl" }}>
          مرحبا بالعالم — hello
        </Text>
      </View>
      <Text style={{ fontSize: 12, color: muted }}>
        paddingStart and borderStartWidth resolve to the right edge inside this subtree.
      </Text>
    </Panel>
  );
}

/** Linear, radial, and conic gradients parsed by the Rust binding into one bounded core value. */
function Gradients() {
  const swatch: Style = {
    height: 64,
    borderRadius: 12,
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
  };
  return (
    <Panel title="Gradients">
      <View style={{ ...swatch, background: "linear-gradient(135deg, #1d4ed8, #38bdf8 60%, #a855f7)" }}>
        <Text style={{ fontSize: 12, fontWeight: 700, color: "#0b1220" }}>linear-gradient</Text>
      </View>
      <View
        style={{
          ...swatch,
          background: "radial-gradient(circle closest-side at 30% 40%, #f8fafc, #0f172a)",
        }}
      >
        <Text style={{ fontSize: 12, fontWeight: 700, color: ink }}>radial-gradient</Text>
      </View>
      <View style={{ ...swatch, background: "conic-gradient(from 200deg, #f97316, #38bdf8, #f97316)" }}>
        <Text style={{ fontSize: 12, fontWeight: 700, color: "#0b1220" }}>conic-gradient</Text>
      </View>
      <View
        style={{
          ...swatch,
          background: {
            type: "linear",
            angle: 90,
            interpolation: "oklab",
            stops: [
              { color: "#0ea5e9", position: 0 },
              { color: "#e879f9", position: 1 },
            ],
          },
        }}
      >
        <Text style={{ fontSize: 12, fontWeight: 700, color: "#0b1220" }}>object form, oklab</Text>
      </View>
    </Panel>
  );
}

/** Per-corner radii, dashed and dotted borders, and outline rings that never affect layout. */
function BordersAndOutlines() {
  return (
    <Panel title="Corners, borders, and outlines">
      <View
        style={{
          height: 56,
          borderRadius: "22px 6px 22px 6px",
          backgroundColor: "#1b2434",
          borderWidth: 1,
          borderColor: "#38bdf8",
        }}
      />
      <View
        style={{
          height: 56,
          borderRadius: 12,
          borderWidth: 2,
          borderColor: "#f97316",
          borderStyle: "dashed",
        }}
      />
      <View
        style={{
          height: 56,
          borderRadius: 12,
          borderWidth: 2,
          borderColor: "#22c55e",
          borderStyle: "dotted",
        }}
      />
      <View
        style={{
          height: 56,
          margin: 6,
          borderRadius: 12,
          backgroundColor: "#1b2434",
          outline: "2px solid #a855f7",
          outlineOffset: 4,
        }}
      />
      <View
        style={{
          height: 56,
          margin: 6,
          borderTopLeftRadius: 28,
          borderBottomRightRadius: 28,
          backgroundColor: "#1b2434",
          outline: "2px dashed #38bdf8",
          outlineOffset: 3,
        }}
      />
    </Panel>
  );
}

/** Colour filters, subtree blur, drop shadows, and a backdrop material. */
function Filters() {
  const tile: Style = {
    height: 52,
    borderRadius: 10,
    background: "linear-gradient(90deg, #f97316, #38bdf8)",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
  };
  return (
    <Panel title="Filters and backdrop">
      <View style={{ ...tile, filter: "saturate(1.8) contrast(1.15)" }}>
        <Text style={{ fontSize: 12, fontWeight: 700, color: "#0b1220" }}>saturate + contrast</Text>
      </View>
      <View style={{ ...tile, filter: "grayscale(1) brightness(1.2)" }}>
        <Text style={{ fontSize: 12, fontWeight: 700, color: "#0b1220" }}>grayscale</Text>
      </View>
      <View style={{ ...tile, filter: "hue-rotate(140deg)" }}>
        <Text style={{ fontSize: 12, fontWeight: 700, color: "#0b1220" }}>hue-rotate</Text>
      </View>
      <View style={{ ...tile, filter: "blur(2px)" }}>
        <Text style={{ fontSize: 12, fontWeight: 700, color: "#0b1220" }}>subtree blur</Text>
      </View>
      <View style={{ ...tile, filter: "drop-shadow(0 6px 12px #0b1220)" }}>
        <Text style={{ fontSize: 12, fontWeight: 700, color: "#0b1220" }}>drop-shadow</Text>
      </View>
      <View
        style={{
          position: "relative",
          height: 84,
          borderRadius: 12,
          background: "conic-gradient(from 30deg, #1d4ed8, #f97316, #1d4ed8)",
          padding: 12,
        }}
      >
        <View
          style={{
            width: "100%",
            height: "100%",
            borderRadius: 10,
            backgroundColor: "#0f172a80",
            backdropFilter: "blur(14px) brightness(1.1)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
          }}
        >
          <Text style={{ fontSize: 12, fontWeight: 700, color: ink }}>backdropFilter</Text>
        </View>
      </View>
    </Panel>
  );
}

/** Paint-only transforms, including the hover variants the core's state styles support. */
function Transforms() {
  const card: Style = {
    height: 54,
    borderRadius: 12,
    backgroundColor: "#1b2434",
    borderColor: panelBorder,
    borderWidth: 1,
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
  };
  return (
    <Panel title="Transforms and blending">
      {/* A style array merges left to right, so a variant is one entry rather than a spread. */}
      <View style={[card, { transform: "rotate(-3deg)" }]}>
        <Text style={{ fontSize: 12, color: ink }}>rotate(-3deg)</Text>
      </View>
      <View style={[card, { transform: "skew(8deg, 0)", transformOrigin: "left center" }]}>
        <Text style={{ fontSize: 12, color: ink }}>skew from the left edge</Text>
      </View>
      <View
        style={{
          ...card,
          transform: "scale(0.96)",
          transition: { property: "background-color", duration: "120ms", easing: "ease-out" },
          hover: {
            transform: "scale(1.03) translate(0, -2px)",
            background: "linear-gradient(90deg, #1d4ed8, #38bdf8)",
            outline: "2px solid #93c5fd",
          },
          outlineOffset: 3,
          cursor: "pointer",
        }}
      >
        <Text style={{ fontSize: 12, color: ink }}>hover to lift, glow, and ring</Text>
      </View>
      <View
        style={{
          height: 84,
          borderRadius: 12,
          background: "linear-gradient(90deg, #f97316, #38bdf8)",
          padding: 10,
          display: "flex",
          flexDirection: "row",
          gap: 10,
        }}
      >
        <View
          style={{
            flex: 1,
            borderRadius: 10,
            backgroundColor: "#f8fafc",
            mixBlendMode: "multiply",
          }}
        />
        <View
          style={{ flex: 1, borderRadius: 10, backgroundColor: "#334155", mixBlendMode: "screen" }}
        />
        <View
          style={{ flex: 1, borderRadius: 10, backgroundColor: "#94a3b8", mixBlendMode: "overlay" }}
        />
      </View>
    </Panel>
  );
}

/**
 * Interaction states the core paints on its own: a hover group revealing its member, a drop zone
 * lighting up for a compatible payload, and a disabled control dimming itself.
 */
function InteractionStates() {
  const rowStyle: Style = {
    display: "flex",
    flexDirection: "row",
    alignItems: "center",
    gap: 10,
    height: 40,
    paddingLeft: 12,
    paddingRight: 8,
    borderRadius: 10,
    backgroundColor: "#1b2434",
    transition: "background-color 120ms",
    hover: { backgroundColor: "#243047" },
    // Tab to a row's button and the row itself shows the ring.
    focusWithin: { outline: "1px solid #93c5fd" },
  };
  const actionStyle: Style = {
    height: 26,
    paddingLeft: 10,
    paddingRight: 10,
    borderRadius: 7,
    backgroundColor: "#2b3a5c",
    opacity: 0,
    transition: "opacity 120ms, background-color 120ms",
    groupHover: { opacity: 1 },
    groupActive: { opacity: 0.7 },
    hover: { backgroundColor: "#3b82f6", transform: "translate(0, -1px)" },
    active: { backgroundColor: "#1d4ed8", transform: "scale(0.97)" },
    focus: { outline: "2px solid #93c5fd" },
    disabled: { opacity: 0.35, cursor: "not-allowed" },
  };
  return (
    <Panel title="Interaction states">
      <View group="list" style={{ display: "flex", flexDirection: "column", gap: 12 }}>
        <For each={["Quarterly report", "Roadmap draft"]}>
          {(title, index) => (
            <View group style={rowStyle}>
              <Text style={{ flex: 1, fontSize: 13, color: ink }}>{title}</Text>
              {/* Follows the whole list, not the row: every hint shows while the list is hovered. */}
              <Text
                style={{
                  fontSize: 11,
                  color: muted,
                  opacity: 0,
                  transition: "opacity 120ms",
                  groupHover: { group: "list", opacity: 1 },
                }}
              >
                {`⌘${index() + 1}`}
              </Text>
              <Button style={actionStyle}>
                <Text style={{ fontSize: 12, color: ink }}>Rename</Text>
              </Button>
              <Button disabled style={actionStyle}>
                <Text style={{ fontSize: 12, color: ink }}>Share</Text>
              </Button>
            </View>
          )}
        </For>
      </View>
      <View
        dropKinds={["files", "local"]}
        draggable={{ id: "swatch", text: "swatch" }}
        style={{
          height: 54,
          borderRadius: 12,
          borderWidth: 1,
          borderStyle: "dashed",
          borderColor: panelBorder,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          transition: "background-color 120ms, border-color 120ms",
          dragging: { opacity: 0.6 },
          dragOver: { borderColor: "#38bdf8", background: "#38bdf826" },
        }}
      >
        <Text style={{ fontSize: 12, color: muted }}>
          hover a row to reveal Rename; drag a file here, or drag this zone itself
        </Text>
      </View>
    </Panel>
  );
}

/** A sticky section header pinned inside its own scroll container. */
function StickyHeaders() {
  const sections = ["Inbox", "Archive", "Trash"];
  return (
    <Panel title="Sticky headers">
      <View
        style={{
          height: 200,
          overflowY: "scroll",
          display: "flex",
          flexDirection: "column",
          borderRadius: 10,
          borderWidth: 1,
          borderColor: panelBorder,
        }}
      >
        <For each={sections}>
          {(section, index) => (
            <View style={{ display: "flex", flexDirection: "column", flexShrink: 0 }}>
              <View
                style={{
                  position: "sticky",
                  top: 0,
                  height: 28,
                  flexShrink: 0,
                  paddingLeft: 12,
                  justifyContent: "center",
                  backgroundColor: index() % 2 === 0 ? "#1e3a5f" : "#3c2858",
                }}
              >
                <Text style={{ fontSize: 12, fontWeight: 700, color: ink }}>{section}</Text>
              </View>
              <For each={[0, 1, 2, 3, 4, 5]}>
                {(row) => (
                  <View
                    style={{
                      height: 30,
                      flexShrink: 0,
                      paddingLeft: 12,
                      justifyContent: "center",
                    }}
                  >
                    <Text style={{ fontSize: 12, color: muted }}>{`${section} row ${row}`}</Text>
                  </View>
                )}
              </For>
            </View>
          )}
        </For>
      </View>
    </Panel>
  );
}

/** A horizontally scrolling rail whose pages snap to their start edge. */
function ScrollSnap() {
  const tints = ["#1d4ed8", "#7c3aed", "#0f766e", "#b45309", "#be123c"];
  return (
    <Panel title="Mandatory scroll snap">
      <View
        style={{
          height: 130,
          overflowX: "scroll",
          scrollSnapType: "x mandatory",
          display: "flex",
          flexDirection: "row",
          gap: 12,
          padding: 6,
          borderRadius: 10,
          borderWidth: 1,
          borderColor: panelBorder,
        }}
      >
        <For each={tints}>
          {(tint, index) => (
            <View
              style={{
                width: 200,
                height: 110,
                flexShrink: 0,
                scrollSnapAlign: "start",
                scrollSnapStop: "always",
                borderRadius: 12,
                backgroundColor: tint,
                alignItems: "center",
                justifyContent: "center",
              }}
            >
              <Text style={{ fontSize: 14, fontWeight: 700, color: "#f8fafc" }}>
                {`page ${index()}`}
              </Text>
            </View>
          )}
        </For>
      </View>
      <Text style={{ fontSize: 12, color: muted }}>
        The core resolves the snap target at the momentum end phase and animates to it on exact
        deadlines, leaving the window settled.
      </Text>
    </Panel>
  );
}

function StylingExample() {
  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        width: "100%",
        height: "100%",
        backgroundColor: "#0b0f17",
        color: ink,
      }}
    >
      <View
        style={{
          height: 52,
          flexShrink: 0,
          justifyContent: "center",
          paddingLeft: 96,
          appRegion: "drag",
        }}
      >
        <Text style={{ fontSize: 14, fontWeight: 700 }}>Declared styling</Text>
      </View>
      <View
        style={{
          flex: 1,
          overflowY: "scroll",
          padding: 20,
          display: "grid",
          gridTemplateColumns: 2,
          gap: 16,
        }}
      >
        <TextAlignment />
        <TextStyling />
        <Direction />
        <Gradients />
        <BordersAndOutlines />
        <Filters />
        <Transforms />
        <InteractionStates />
        <StickyHeaders />
        <ScrollSnap />
      </View>
    </View>
  );
}

function openMainWindow() {
  new Window({
    title: "QuickGUI Styling",
    width: 960,
    height: 720,
    minimumWidth: 720,
    minimumHeight: 560,
    background: "#0b0f17",
    titleBarStyle: "hiddenInset",
    trafficLightPosition: { x: 16, y: 18 },
    renderer: createRenderer(() => <StylingExample />),
  });
}

app.onReopen((event) => {
  if (!event.hasVisibleWindows) openMainWindow();
});
openMainWindow();
