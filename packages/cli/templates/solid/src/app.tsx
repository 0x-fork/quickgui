import { App, Button, Text, View, Window, render } from "@quickgui/solid";
import { createSignal } from "solid-js";

const app = new App();
const window = new Window({
  title: {{APP_NAME}},
  width: 720,
  height: 480,
  background: "#0b1020",
});

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

render(() => <Counter />, window);
await app.run();
