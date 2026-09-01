import { app, Window } from "@quickgui/native";
import type {
  MacOSVibrancy,
  MacOSVisualEffectState,
} from "@quickgui/native";
import { Button, Text, View, createRenderer } from "@quickgui/solid";
import { createSignal } from "solid-js";

const vibrancyTypes = [
  ["appearance-based", "Appearance based"],
  ["titlebar", "Titlebar"],
  ["selection", "Selection"],
  ["menu", "Menu"],
  ["popover", "Popover"],
  ["sidebar", "Sidebar"],
  ["header", "Header"],
  ["sheet", "Sheet"],
  ["window", "Window"],
  ["hud", "HUD"],
  ["fullscreen-ui", "Fullscreen UI"],
  ["tooltip", "Tooltip"],
  ["content", "Content"],
  ["under-window", "Under window"],
  ["under-page", "Under page"],
] as const satisfies readonly (readonly [MacOSVibrancy, string])[];

const visualEffectStates = [
  ["followWindow", "Auto"],
  ["active", "Active"],
  ["inactive", "Inactive"],
] as const satisfies readonly (readonly [MacOSVisualEffectState, string])[];

await app.whenReady();

function openMainWindow() {
  new Window({
    title: "QuickGUI Sidebar Vibrancy",
    width: 860,
    height: 620,
    minimumWidth: 700,
    minimumHeight: 480,
    background: "transparent",
    vibrancy: "sidebar",
    visualEffectState: "followWindow",
    appearance: "light",
    titleBarStyle: "hiddenInset",
    trafficLightPosition: { x: 16, y: 19 },
    renderer: createRenderer(() => <SidebarVibrancyExample />),
  });
}

app.on("reopen", ({ hasVisibleWindows }) => {
  if (!hasVisibleWindows) openMainWindow();
});
openMainWindow();

function SidebarVibrancyExample() {
  const window = Window.getCurrentWindow();
  const [vibrancy, setVibrancy] = createSignal<MacOSVibrancy>("sidebar");
  const [effectState, setEffectState] =
    createSignal<MacOSVisualEffectState>("followWindow");

  function selectVibrancy(value: MacOSVibrancy) {
    setVibrancy(value);
    window.setVibrancy(value);
  }

  function selectEffectState(value: MacOSVisualEffectState) {
    setEffectState(value);
    window.setVisualEffectState(value);
  }

  return (
    <View
      style={{
        display: "flex",
        flexDirection: "row",
        width: "100%",
        height: "100%",
        backgroundColor: "transparent",
        color: "#172033",
      }}
    >
      <View
        style={{
          display: "flex",
          flexDirection: "column",
          width: 254,
          height: "100%",
          flexShrink: 0,
          backgroundColor: "transparent",
          borderColor: "#cccccc",
          borderRightWidth: 1,
        }}
      >
        <View
          style={{
            display: "flex",
            height: 52,
            flexShrink: 0,
            alignItems: "center",
            paddingLeft: 90,
            paddingRight: 14,
            appRegion: "drag",
          }}
        >
          <Text style={{ fontSize: 13, fontWeight: 700 }}>Vibrancy</Text>
        </View>

        <View
          style={{
            display: "flex",
            flexDirection: "column",
            flex: 1,
            minHeight: 0,
            gap: 3,
            paddingLeft: 10,
            paddingRight: 10,
            paddingBottom: 10,
            overflowY: "auto",
          }}
        >
          {vibrancyTypes.map(([value, label]) => (
            <Button
              onClick={() => selectVibrancy(value)}
              style={{
                display: "flex",
                flexDirection: "row",
                width: "100%",
                height: 31,
                flexShrink: 0,
                alignItems: "center",
                justifyContent: "space-between",
                paddingLeft: 11,
                paddingRight: 11,
                backgroundColor: vibrancy() === value ? "#ffffff52" : "transparent",
                borderColor: vibrancy() === value ? "#ffffff70" : "transparent",
                borderWidth: 1,
                borderRadius: 7,
                color: "#263247",
                cursor: "default",
                appRegion: "no-drag",
                userSelect: "none",
              }}
            >
              <Text style={{ fontSize: 12, fontWeight: 600 }}>{label}</Text>
              {vibrancy() === value ? (
                <Text style={{ color: "#2563eb", fontSize: 12, fontWeight: 700 }}>✓</Text>
              ) : null}
            </Button>
          ))}
        </View>

        <View
          style={{
            display: "flex",
            flexDirection: "column",
            flexShrink: 0,
            gap: 7,
            padding: 12,
            borderColor: "#c1c1c2",
            borderTopWidth: 1,
          }}
        >
          <Text style={{ color: "#59667b", fontSize: 11, fontWeight: 700 }}>EFFECT STATE</Text>
          <View style={{ display: "flex", flexDirection: "row", gap: 5 }}>
            {visualEffectStates.map(([value, label]) => (
              <Button
                onClick={() => selectEffectState(value)}
                style={{
                  display: "flex",
                  flex: 1,
                  height: 27,
                  minWidth: 0,
                  alignItems: "center",
                  justifyContent: "center",
                  backgroundColor: effectState() === value ? "#ffffff5c" : "#ffffff24",
                  borderColor: effectState() === value ? "#ffffff7a" : "#ffffff3d",
                  borderWidth: 1,
                  borderRadius: 6,
                  color: "#445168",
                  fontSize: 10,
                  fontWeight: 600,
                  cursor: "default",
                  appRegion: "no-drag",
                  userSelect: "none",
                }}
              >
                {label}
              </Button>
            ))}
          </View>
        </View>
      </View>

      <View
        style={{
          display: "flex",
          flexDirection: "column",
          flex: 1,
          minWidth: 0,
          height: "100%",
          backgroundColor: "#ffffff",
        }}
      >
        <View
          style={{
            display: "flex",
            height: 52,
            flexShrink: 0,
            alignItems: "center",
            justifyContent: "center",
            borderColor: "#e2e8f0",
            borderBottomWidth: 1,
            appRegion: "drag",
          }}
        >
          <Text style={{ fontSize: 13, fontWeight: 700 }}>{vibrancy()}</Text>
        </View>

        <View
          style={{
            display: "flex",
            flex: 1,
            minHeight: 0,
            alignItems: "center",
            justifyContent: "center",
            padding: 36,
          }}
        >
          <View
            style={{
              display: "flex",
              flexDirection: "column",
              width: "100%",
              maxWidth: 480,
              gap: 16,
              padding: 28,
              backgroundColor: "#ffffff",
              borderColor: "#dfe5ed",
              borderWidth: 1,
              borderRadius: 14,
              boxShadow: "0 18px 45px -24px rgba(15, 23, 42, 0.35)",
            }}
          >
            <Text style={{ color: "#2563eb", fontSize: 12, fontWeight: 700 }}>
              SOLID + QUICKGUI
            </Text>
            <Text style={{ fontSize: 26, lineHeight: 33, fontWeight: 700 }}>
              Every macOS vibrancy type
            </Text>
            <Text style={{ color: "#667085", fontSize: 14, lineHeight: 21 }}>
              Select any Electron-compatible semantic material. QuickGUI updates one native
              NSVisualEffectView while the retained Solid tree and Metal surface stay mounted.
              This pane is opaque, so the selected material remains visually confined to the
              translucent sidebar.
            </Text>
            <View
              style={{
                display: "flex",
                flexDirection: "row",
                gap: 10,
                paddingTop: 4,
              }}
            >
              <Readout label="Material" value={vibrancy()} />
              <Readout label="Effect state" value={effectState()} />
              <Readout label="Content" value="Opaque" />
            </View>
          </View>
        </View>
      </View>
    </View>
  );
}

function Readout(props: { label: string; value: string }) {
  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        flex: 1,
        minWidth: 0,
        gap: 3,
        padding: 12,
        backgroundColor: "#f8fafc",
        borderColor: "#e2e8f0",
        borderWidth: 1,
        borderRadius: 9,
      }}
    >
      <Text style={{ color: "#8a94a6", fontSize: 11 }}>{props.label}</Text>
      <Text style={{ color: "#354056", fontSize: 11, fontWeight: 700 }}>{props.value}</Text>
    </View>
  );
}
