import { join } from "node:path";

import { Window, app } from "@quickgui/native";
import { View, createRenderer } from "@quickgui/solid";

import { AgentSheet } from "./components/agent-sheet.tsx";
import { Sidebar } from "./components/sidebar.tsx";
import { Workspace } from "./components/workspace.tsx";
import { createHerdrModel } from "./controller.ts";
import {
  normalizeSpaces,
  readSavedState,
} from "./services/persistence.ts";
import { themeFor } from "./theme.ts";

await app.whenReady();

const appPaths = await app.getPaths();
const homePath = appPaths?.homeDir ?? process.cwd();
const stateFile = appPaths?.dataDir
  ? join(appPaths.dataDir, "herdr-gui-state.json")
  : appPaths?.configDir
    ? join(appPaths.configDir, "herdr-gui-state.json")
    : undefined;
const savedState = await readSavedState(stateFile);
const restoredSpaces = normalizeSpaces(savedState.spaces, homePath);
const initialAppearance = savedState.appearance ?? "system";

function HerdrGui() {
  const model = createHerdrModel({
    window: Window.getCurrentWindow(),
    homePath,
    stateFile,
    savedState,
    restoredSpaces,
    initialAppearance,
  });
  return (
    <View style={model.styles().app}>
      <Sidebar model={model} />
      <Workspace model={model} />
      <AgentSheet model={model} />
    </View>
  );
}

function openMainWindow() {
  new Window({
    title: "Herdr GUI",
    width: 1220,
    height: 780,
    minimumWidth: 860,
    minimumHeight: 560,
    background: themeFor(initialAppearance === "light" ? "light" : "dark").app,
    appearance: initialAppearance,
    titleBarStyle: "hiddenInset",
    trafficLightPosition: { x: 15, y: 14 },
    renderer: createRenderer(() => <HerdrGui />),
  });
}

app.on("reopen", ({ hasVisibleWindows }) => {
  if (!hasVisibleWindows) openMainWindow();
});
openMainWindow();
