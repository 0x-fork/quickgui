import { createSignal, Show } from "solid-js";
import { app, Window } from "@quickgui/native";
import { Button, Text, TextInput, View, createRenderer } from "@quickgui/solid";

function Counter() {
  const [count, setCount] = createSignal(0);
  const [name, setName] = createSignal("Bun + Solid 2");
  return (
    <View
      style={{
        width: "100%",
        height: "100%",
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        gap: 18,
        bg: "#18181b",
        textColor: "#fafafa",
      }}
    >
      <Text style={{ fontSize: 28, fontWeight: 600 }}>{name()}</Text>
      <TextInput
        value={name()}
        onInput={(event) => setName(event.value ?? "")}
        style={{ width: 250, padding: 10, borderRadius: 8, bg: "#27272a" }}
      />
      <Text style={{ fontSize: 20 }}>Count: {count()}</Text>
      <View style={{ display: "flex", gap: 12 }}>
        <Button onClick={() => setCount(0)} style={{ padding: 12, borderRadius: 8, bg: "#3f3f46" }}>
          Reset
        </Button>
        <Button
          onClick={() => setCount(count() + 1)}
          style={{ padding: 12, borderRadius: 8, bg: "#2563eb" }}
        >
          Increment
        </Button>
      </View>
      <Show when={count() >= 5}>
        <Text style={{ textColor: "#86efac" }}>Five clicks! This branch is reactive.</Text>
      </Show>
    </View>
  );
}

function openWindow() {
  return new Window({
    title: "TypeScript Counter",
    width: 640,
    height: 460,
    renderer: createRenderer(Counter),
  });
}
app.on("reopen", ({ hasVisibleWindows }) => {
  if (!hasVisibleWindows) openWindow();
});
await app.whenReady();
openWindow();
