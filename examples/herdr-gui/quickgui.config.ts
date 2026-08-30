import { defineConfig } from "@quickgui/cli";

export default defineConfig({
  name: "Herdr GUI",
  identifier: "dev.quickgui.herdr-gui-example",
  entry: "app.tsx",
  fonts: [
    "assets/JetBrainsMonoNerdFontMono-Regular.ttf",
    "assets/JetBrainsMonoNerdFontMono-Bold.ttf",
    "assets/JetBrainsMonoNerdFontMono-Italic.ttf",
    "assets/JetBrainsMonoNerdFontMono-BoldItalic.ttf",
  ],
  resources: ["assets/JetBrainsMonoNerdFont-OFL.txt"],
});
