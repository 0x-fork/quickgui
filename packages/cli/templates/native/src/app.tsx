import { Window, app, type NativeNode } from "@quickgui/native";
import { Button, Text, View, createRenderer, createSignal } from "@quickgui/ui";

await app.whenReady();

function openMainWindow(): void {
  new Window({
    title: {{APP_NAME}},
    width: 640,
    height: 420,
    background: "#fafafa",
    renderer: createRenderer(() => <Counter />),
  });
}

app.onReopen((event) => {
  if (!event.hasVisibleWindows) openMainWindow();
});
openMainWindow();

function Counter(): NativeNode {
  const [count, setCount] = createSignal(0);
  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        width: "100%",
        height: "100%",
        alignItems: "center",
        justifyContent: "center",
        gap: 16,
        color: "#18181b",
      }}
    >
      <Text style={{ fontSize: 24, fontWeight: 600 }}>Count: {count()}</Text>
      <Button
        onClick={() => setCount(count() + 1)}
        style={{
          display: "flex",
          height: 40,
          paddingLeft: 20,
          paddingRight: 20,
          alignItems: "center",
          justifyContent: "center",
          backgroundColor: "#18181b",
          color: "#fafafa",
          borderRadius: 8,
          cursor: "default",
          userSelect: "none",
          hover: { backgroundColor: "#3f3f46" },
        }}
      >
        Increment
      </Button>
    </View>
  );
}
