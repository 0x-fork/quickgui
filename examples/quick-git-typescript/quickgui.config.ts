import { defineConfig } from "@quickgui/cli";

export default defineConfig({
  frontend: "typescript",
  name: "Quick Git",
  identifier: "dev.quickgui.quick-git.typescript",
  entry: "app.tsx",
  macos: {
    category: "public.app-category.developer-tools",
  },
});
