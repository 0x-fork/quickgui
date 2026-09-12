import { app, SystemPreferences, Window } from "@quickgui/native";
import { createRenderer } from "@quickgui/solid";
import { CodeMorphDemo } from "./demo.tsx";
import { loadSnippets } from "./snippets.ts";

const snippets = await loadSnippets();
await app.whenReady();

async function openWindow() {
  const preferences = await SystemPreferences.getCurrent();
  return new Window({
    title: "Code Morph",
    width: 1060,
    height: 930,
    minimumWidth: 860,
    minimumHeight: 600,
    appearance: "dark",
    background: "#0d1117",
    renderer: createRenderer(() => (
      <CodeMorphDemo snippets={snippets} reduceMotion={preferences.reduceMotion === true} />
    )),
  });
}

app.on("reopen", ({ hasVisibleWindows }) => {
  if (!hasVisibleWindows) void openWindow();
});
await openWindow();
