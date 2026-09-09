import { createSignal, Show } from "solid-js";
import { app, Window } from "@quickgui/native";
import { Button, Text, TextInput, View, createRenderer } from "@quickgui/solid";

function Counter() {
  const [count, setCount] = createSignal(0);
  const [name, setName] = createSignal("Bun + Solid 2");
  return (
    <View
      width="100%"
      height="100%"
      display="flex"
      flexDirection="column"
      alignItems="center"
      justifyContent="center"
      gap={18}
      backgroundColor="#18181b"
      textColor="#fafafa"
    >
      <Text fontSize={28} fontWeight={600}>
        {name()}
      </Text>
      <TextInput
        width={250}
        padding={10}
        borderRadius={8}
        backgroundColor="#27272a"
        value={name()}
        onInput={(event) => setName(event.value ?? "")}
      />
      <Text fontSize={20}>Count: {count()}</Text>
      <View display="flex" gap={12}>
        <Button padding={12} borderRadius={8} backgroundColor="#3f3f46" onClick={() => setCount(0)}>
          Reset
        </Button>
        <Button
          padding={12}
          borderRadius={8}
          backgroundColor="#2563eb"
          onClick={() => setCount(count() + 1)}
        >
          Increment
        </Button>
      </View>
      <Show when={count() >= 5}>
        <Text textColor="#86efac">Five clicks! This branch is reactive.</Text>
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
