import { DOCS_GUIDE_ORDER, docsOutline, docsTitle } from "./docs-structure";
import type { DocsPageMeta } from "./docs";

const descriptions = {
  "getting-started": "Build native desktop apps with TypeScript, Bun, and Solid 2.",
  "project-structure": "Configure TypeScript projects, JSX compilation, and native packaging.",
  updater: "Use the optional Rust updater extension and Sparkle on macOS.",
  extensions: "Share Solid components and load native service or terminal extensions.",
  "native-services": "Manage windows, menus, dialogs, clipboard, and asynchronous native work.",
  reactivity: "Use Solid 2 signals, memos, effects, and cleanup with retained native nodes.",
  rendering: "Understand native JSX, keyed identity, window ownership, and testing.",
  components: "Compose the complete QuickGUI component families using Solid JSX.",
  routing: "Render nested native layouts with Rust-owned route matching and history.",
  styling: "Apply layout, typography, paint, and interaction styles to native components.",
  animations: "Use native transitions and animated images without JavaScript frame polling.",
  "forms-and-input": "Build controlled text inputs, fields, choices, and pickers.",
  "overlays-and-dialogs": "Compose popovers, dialogs, native panels, and file dialogs.",
  "swift-ui": "Embed real SwiftUI controls inside the retained QuickGUI renderer.",
  "swift-ui-hosting": "Style native controls and host QuickGUI content inside SwiftUI.",
};
export const TYPESCRIPT_DOCS_PAGES: readonly DocsPageMeta[] = DOCS_GUIDE_ORDER.map((slug) => ({
  frontend: "typescript",
  slug,
  title: docsTitle(slug),
  outline: docsOutline(slug),
  description: descriptions[slug],
  searchTerms: ["typescript", "bun", "solid", "jsx", "ffi", slug],
  translations: {
    zh: { description: `使用 TypeScript、Bun 和 Solid 2：${docsTitle(slug, "zh")}。` },
    ja: { description: `TypeScript、Bun、Solid 2 での${docsTitle(slug, "ja")}。` },
  },
}));
