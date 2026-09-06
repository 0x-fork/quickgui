import { Window, app, type NativeNode } from "@quickgui/native";
import { Button, Show, Text, View, createRenderer, createSignal } from "@quickgui/ui";

await app.whenReady();

function openMainWindow(): void {
  new Window({
    title: "QuickGUI Counter",
    width: 760,
    height: 520,
    minimumWidth: 520,
    minimumHeight: 360,
    background: "#090d16",
    titleBarStyle: "hiddenInset",
    trafficLightPosition: { x: 16, y: 13 },
    renderer: createRenderer(() => <Counter />),
  });
}

app.onReopen((event) => {
  if (!event.hasVisibleWindows) openMainWindow();
});
openMainWindow();

function openDetailsWindow(): void {
  new Window({
    title: "Dynamic QuickGUI window",
    width: 420,
    height: 260,
    minimumWidth: 320,
    minimumHeight: 200,
    background: "#111827",
    renderer: createRenderer(() => {
      const window = Window.getCurrentWindow();
      return (
        <View
          style={{
            display: "flex",
            flexDirection: "column",
            width: "100%",
            height: "100%",
            justifyContent: "center",
            gap: 16,
            padding: 28,
            backgroundColor: "#111827",
            color: "#e2e8f0",
          }}
        >
          <Text style={{ fontSize: 22, fontWeight: 700 }}>Created while the app is running</Text>
          <Text style={{ color: "#94a3b8", lineHeight: 21 }}>
            This window has its own retained tree and native lifecycle.
          </Text>
          <Button
            onClick={() => window.close()}
            style={{
              display: "flex",
              height: 40,
              alignItems: "center",
              justifyContent: "center",
              backgroundColor: "#334155",
              borderRadius: 9,
              cursor: "default",
            }}
          >
            Close window
          </Button>
        </View>
      );
    }),
  });
}

function Counter(): NativeNode {
  const [count, setCount] = createSignal(0);

  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        width: "100%",
        height: "100%",
        backgroundColor: "#090d16",
        color: "#e2e8f0",
      }}
    >
      <View
        style={{
          display: "flex",
          height: 52,
          flexShrink: 0,
          alignItems: "center",
          justifyContent: "center",
          appRegion: "drag",
          borderColor: "#1e293b",
          borderWidth: 1,
        }}
      >
        <Text style={{ fontWeight: 600 }}>QuickGUI · native</Text>
      </View>

      <View
        style={{
          display: "flex",
          flex: 1,
          minHeight: 0,
          alignItems: "center",
          justifyContent: "center",
          padding: 32,
        }}
      >
        <View
          style={{
            display: "flex",
            flexDirection: "column",
            width: 420,
            gap: 18,
            padding: 28,
            backgroundColor: "#111827",
            borderColor: "#334155",
            borderWidth: 1,
            borderRadius: 16,
          }}
        >
          <Text style={{ fontSize: 28, lineHeight: 36, fontWeight: 700 }}>Fine-grained native UI</Text>
          <Text style={{ color: "#94a3b8", fontSize: 14, lineHeight: 21 }}>
            Signals update only the changed text node. The application is compiled to native code,
            and QuickGUI retains layout, sleeps while clean, and redraws once per mutation batch.
          </Text>
          <Text style={{ color: "#bfdbfe", fontSize: 20, fontWeight: 600 }}>Count: {count()}</Text>
          <Show when={count() >= 5}>
            <Text style={{ color: "#fbbf24", fontSize: 14 }}>Five or more clicks: the row above was created on demand.</Text>
          </Show>
          <Button
            onClick={() => setCount(count() + 1)}
            style={{
              display: "flex",
              height: 44,
              alignItems: "center",
              justifyContent: "center",
              backgroundColor: "#2563eb",
              color: "white",
              borderRadius: 9,
              cursor: "default",
              appRegion: "no-drag",
              userSelect: "none",
              hover: { backgroundColor: "#3b82f6" },
            }}
          >
            Increment
          </Button>
          <Button
            onClick={openDetailsWindow}
            style={{
              display: "flex",
              height: 44,
              alignItems: "center",
              justifyContent: "center",
              backgroundColor: "#334155",
              color: "white",
              borderRadius: 9,
              cursor: "default",
              appRegion: "no-drag",
              userSelect: "none",
            }}
          >
            Open window
          </Button>
        </View>
      </View>
    </View>
  );
}
