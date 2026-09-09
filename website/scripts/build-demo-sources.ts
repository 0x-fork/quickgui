import { existsSync, readFileSync, mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { createHighlighter } from "shiki";
import type { DemoSource } from "../src/lib/demo-source";

const root = resolve(import.meta.dir, "../..");
const entryPath = "crates/quickgui-docs-demo/src/lib.rs";
const destination = resolve(root, "website/src/lib/generated/demo-sources.json");
const galleries: Record<string, [string, string[]]> = {
  avatar: ["base_ui_components", ["avatar_section"]],
  "checkbox-group": ["base_ui_components", ["checkbox_group_section"]],
  "preview-card": ["base_ui_components", ["preview_card_section"]],
  "scroll-area": ["base_ui_components", ["scroll_area_section"]],
  "otp-field": ["base_ui_components", ["otp_section"]],
  "navigation-menu": ["base_ui_components", ["navigation_section"]],
  "number-field": ["range_controls", ["number_field"]],
  splitter: ["range_controls", ["splitter"]],
  toolbar: ["toolbar_toast", ["commands", "chrome"]],
  "toggle-group": ["toolbar_toast", ["alignment", "chrome"]],
  toast: ["toolbar_toast", ["commands", "toasts_surface", "chrome"]],
  "date-field": ["date_fields", ["render"]],
  "time-field": ["date_fields", ["render"]],
  calendar: ["date_fields", ["render"]],
  tabs: ["tabs", ["render"]],
  dialog: ["dialogs", ["gallery_button", "render"]],
  "alert-dialog": ["dialogs", ["gallery_button", "render"]],
  accordion: ["disclosures", ["render", "accordion_item"]],
  table: ["data_collections", ["render"]],
  tree: ["data_collections", ["render"]],
  tooltip: ["tooltips_context_menu", ["hint_row", "render"]],
  menubar: ["menubar", ["render"]],
  popover: ["popovers", ["render", "gallery_trigger"]],
  menu: ["popovers", ["render", "gallery_trigger"]],
};
const aliases: Record<string, string> = {
  "text-area": "input",
  radio: "checkbox",
  switch: "checkbox",
  toggle: "checkbox",
  meter: "progress",
  fieldset: "field",
};

// Extract complete functions from rustfmt-formatted source. Closing braces at the
// declaration's indentation delimit the item; nested blocks retain deeper indentation.
// Fail closed when a function is renamed instead of publishing a stale example.
export function rustFunction(source: string, name: string): string {
  const header = new RegExp(`^( *)fn ${name}(?:<[^\\n]+>)?\\(`, "m").exec(source);
  if (!header) throw new Error(`Missing demo source function: ${name}`);
  const start = header.index;
  const indentation = header[1];
  const end = new RegExp(`^${indentation}\\}`, "m").exec(source.slice(start + header[0].length));
  if (!end) throw new Error(`Missing end of demo source function: ${name}`);
  const text = source.slice(start, start + header[0].length + end.index + end[0].length);
  return text
    .split("\n")
    .map((line) => (line.startsWith(indentation) ? line.slice(indentation.length) : line))
    .join("\n");
}

export async function buildDemoSources(check = false) {
  const entry = readFileSync(resolve(root, entryPath), "utf8");
  const ids = [
    ...entry.match(/pub const COMPONENTS:[\s\S]*?= &\[([\s\S]*?)\];/)![1].matchAll(/"([a-z-]+)"/g),
  ].map((match) => match[1]);
  const highlighter = await createHighlighter({
    langs: ["rust"],
    themes: ["github-light-high-contrast", "github-dark-high-contrast"],
  });
  const result: Record<string, DemoSource> = {};
  for (const component of ids) {
    const gallery = galleries[component];
    const path = gallery ? `examples/${gallery[0]}.rs` : entryPath;
    const source = gallery ? readFileSync(resolve(root, path), "utf8") : entry;
    const functions = gallery?.[1] ?? [
      `preview_${(aliases[component] ?? component).replaceAll("-", "_")}`,
    ];
    const snippets = functions.map((name) => rustFunction(source, name));
    if (!gallery && snippets[0].includes("Self::action("))
      snippets.push(rustFunction(entry, "action"));
    const code = snippets.join("\n\n");
    result[component] = {
      path,
      functions,
      code,
      html: highlighter
        .codeToHtml(code, {
          lang: "rust",
          themes: { light: "github-light-high-contrast", dark: "github-dark-high-contrast" },
          defaultColor: false,
        })
        .replace("<pre ", '<pre tabindex="0" '),
    };
  }
  highlighter.dispose();
  const output = JSON.stringify(result) + "\n";
  if (check) {
    if (readFileSync(destination, "utf8") !== output)
      throw new Error("Preview source is stale. Run bun run docs:wasm.");
  } else {
    mkdirSync(resolve(destination, ".."), { recursive: true });
    if (!existsSync(destination) || readFileSync(destination, "utf8") !== output)
      writeFileSync(destination, output);
  }
  console.log(`Preview source: ${ids.length} demos, extracted from their compiled Rust functions.`);
}
if (import.meta.main) await buildDemoSources(process.argv.includes("--check"));
