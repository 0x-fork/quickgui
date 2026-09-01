import { app, Window } from "@quickgui/native";
import { Button, Text, View, createRenderer } from "@quickgui/solid";
import { createSignal } from "solid-js";

function openMainWindow() {
  new Window({
    title: {{APP_NAME}},
    width: 720,
    height: 480,
    background: "#0b1020",
    renderer: createRenderer(() => <Counter />),
  });
}

app.on("reopen", ({ hasVisibleWindows }) => {
  if (!hasVisibleWindows) openMainWindow();
});

await app.whenReady();
openMainWindow();

function Counter() {
  const [count, setCount] = createSignal(0);

  return (
    <View
      style={{
        display: "flex",
        width: "100%",
        height: "100%",
        alignItems: "center",
        justifyContent: "center",
        background: "#0b1020",
        color: "#f8fafc",
      }}
    >
      <View style={{ display: "flex", flexDirection: "column", width: 360, gap: 16 }}>
        <Text style={{ fontSize: 28, fontWeight: 700 }}>QuickGUI + Solid</Text>
        <Text style={{ color: "#94a3b8" }}>Count: {count()}</Text>
        <Button
          onClick={() => setCount((value) => value + 1)}
          style={{
            height: 44,
            alignItems: "center",
            justifyContent: "center",
            background: "#2563eb",
            borderRadius: 9,
            cursor: "default",
            appRegion: "no-drag",
          }}
        >
          Increment
        </Button>
      </View>
    </View>
  );
}
