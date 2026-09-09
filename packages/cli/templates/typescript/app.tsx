import { createSignal } from "solid-js";
import { app, Window } from "@quickgui/native";
import { Button, Text, View, createRenderer } from "@quickgui/solid";

function Counter() {
  const [count, setCount] = createSignal(0);
  return (
    <View width="100%" height="100%" display="flex" flexDirection="column"
      alignItems="center" justifyContent="center" gap={16}>
      <Text fontSize={24}>Count: {count()}</Text>
      <Button padding={12} onClick={() => setCount(count() + 1)}>Increment</Button>
    </View>
  );
}

function openWindow() {
  new Window({ title: {{APP_NAME}}, width: 640, height: 460, renderer: createRenderer(Counter) });
}
app.on("reopen", ({ hasVisibleWindows }) => { if (!hasVisibleWindows) openWindow(); });
await app.whenReady();
openWindow();
