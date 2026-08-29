import { defineConfig } from "@quickgui/cli";

export default defineConfig({
  name: "QuickGUI System APIs",
  identifier: "dev.quickgui.system-api-example",
  entry: "app.tsx",
  protocols: ["quickgui-system"],
});
