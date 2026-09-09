#!/usr/bin/env bun
/** Generate Go and TypeScript layout conveniences from the Rust Element API. */
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const read = (path: string) => readFileSync(resolve(root, path), "utf8");
const check = process.argv.includes("--check");
const rustElement = read("src/element.rs");
const spacing = Number(/const SPACING_UNIT: f32 = ([\d.]+);/.exec(rustElement)![1]);
const maxRadius = Number(/const MAX_CORNER_RADIUS: f32 = ([\d_.]+);/.exec(rustElement)![1]!.replaceAll("_", ""));
type Method = { name: string; parameters: string; body: string; file: string };
function rustMethods(file: string): Method[] {
  const source = read(file);
  return [...source.matchAll(/pub fn (\w+)([^{}]*?)\{/g)].flatMap((match) => {
    if (!/-> Self\s*$/.test(match[2]!)) return [];
    let end = match.index! + match[0].length, depth = 1;
    const start = end;
    while (depth) {
      const char = source[end++];
      if (char === "{") depth++;
      if (char === "}") depth--;
    }
    return [{ name: match[1]!, parameters: match[2]!, body: source.slice(start, end - 1).trim(), file }];
  });
}
const layoutFile = "src/element/construction_layout.rs";
const layout = rustMethods(layoutFile);
const visual = rustMethods("src/element/style.rs").filter(({ name }) =>
  /^(rounded(?:_|$)|border_(?:solid|dashed|dotted)$|text_(?:xs|sm|base|lg|xl|2xl|3xl|left|center|right|justify|start|end)$|font_(?:normal|medium|semibold|bold)$|whitespace_|text_ellipsis|truncate$|overflow_(?:hidden|y_scroll)$|absolute$|relative$|inset_0$|app_region_(?:drag|no_drag)$)/.test(name));
const cursors = rustMethods("src/element/interaction.rs").filter(({ name }) => /^(cursor_|user_select_|selectable$)/.test(name));
const methods = new Map([...layout, ...visual, ...cursors].map((method) => [method.name, method]));
for (const macro of read(layoutFile).matchAll(/spacing_scale_methods!\((\w+);([\s\S]*?)\);/g)) {
  for (const item of macro[2]!.matchAll(/(\w+)\s*=>\s*([\d.]+)/g)) {
    methods.set(item[1]!, { name: item[1]!, parameters: "(self) -> Self", body: `self.${macro[1]}(${Number(item[2]) * spacing})`, file: layoutFile });
  }
}
type Value = null | string | number | boolean | { arg: string } | { kind: "percent"; arg: string } | { kind: "tracks"; arg: string; min: string; max: string };
type Op = [string, Value];
type Parameter = { name: string; type: "value" | "number" | "count" | "string" };
type Helper = { name: string; parameters: Parameter[]; ops: Op[] };
const helpers = new Map<string, Helper>();
const arg = (name = "value"): Value => ({ arg: name });
function add(name: string, ops: Op[], parameters: Parameter[] = []) {
  if (!methods.has(name)) throw new Error(`No Rust layout helper ${name}`);
  helpers.set(name, { name, ops, parameters });
}
const valueParam: Parameter[] = [{ name: "value", type: "value" }];
const edges = ["top", "right", "bottom", "left"];
const aliases: Record<string, string[]> = {
  p: edges.map((edge) => `padding_${edge}`), px: ["padding_left", "padding_right"], py: ["padding_top", "padding_bottom"],
  m: edges.map((edge) => `margin_${edge}`), mx: ["margin_left", "margin_right"], my: ["margin_top", "margin_bottom"],
  mt: ["margin_top"], mr: ["margin_right"], mb: ["margin_bottom"], ml: ["margin_left"],
  w: ["width"], h: ["height"], min_w: ["min_width"], min_h: ["min_height"], max_w: ["max_width"], max_h: ["max_height"],
  gap_x: ["column_gap"], gap_y: ["row_gap"], ps: ["padding_start"], pe: ["padding_end"], ms: ["margin_start"], me: ["margin_end"],
  border_s: ["border_start_width"], border_e: ["border_end_width"], rounded: ["border_radius"],
  rounded_tl: ["border_top_left_radius"], rounded_tr: ["border_top_right_radius"], rounded_bl: ["border_bottom_left_radius"], rounded_br: ["border_bottom_right_radius"],
  rounded_t: ["border_top_left_radius", "border_top_right_radius"], rounded_b: ["border_bottom_left_radius", "border_bottom_right_radius"],
  rounded_l: ["border_top_left_radius", "border_bottom_left_radius"], rounded_r: ["border_top_right_radius", "border_bottom_right_radius"],
  snap_align: ["scroll_snap_align"],
};
for (const [name, fields] of Object.entries(aliases)) add(name, fields.map((field) => [field, arg()]), name === "snap_align" ? [{ name: "value", type: "string" }] : valueParam);
helpers.get("rounded")!.ops.push(...["top_left", "top_right", "bottom_left", "bottom_right"].map((corner): Op => [`border_${corner}_radius`, null]));
add("size", [["width", arg("width")], ["height", arg("height")]], [{ name: "width", type: "value" }, { name: "height", type: "value" }]);
add("size_full", [["width", "100%"], ["height", "100%"]]);
for (const prefix of ["w", "h"]) {
  const field = prefix === "w" ? "width" : "height";
  add(`${prefix}_full`, [[field, "100%"]]);
  add(`${prefix}_fraction`, [[field, { kind: "percent", arg: "fraction" }]], [{ name: "fraction", type: "number" }]);
}
for (const prefix of ["m", "mx", "my", "mt", "mr", "mb", "ml"]) add(`${prefix}_auto`, aliases[prefix]!.map((field) => [field, "auto"]));
for (const axis of ["cols", "rows"]) {
  for (const [suffix, min, max] of [["", "0", "1fr"], ["_min_content", "min-content", "1fr"], ["_max_content", "0", "max-content"]]) {
    add(`grid_${axis}${suffix}`, [[axis === "cols" ? "grid_template_columns" : "grid_template_rows", { kind: "tracks", arg: "count", min: min!, max: max! }]], [{ name: "count", type: "count" }]);
  }
}
for (const [short, axis] of [["col", "column"], ["row", "row"]]) {
  for (const edge of ["start", "end"]) {
    add(`${short}_${edge}`, [[`grid_${axis}_${edge}`, arg()]], valueParam);
    add(`${short}_${edge}_auto`, [[`grid_${axis}_${edge}`, 0]]);
  }
  add(`${short}_span`, [[`grid_${axis}_start`, null], [`grid_${axis}_end`, null], [`grid_${axis}_span`, arg()]], valueParam);
  add(`${short}_span_full`, [[`grid_${axis}_span`, null], [`grid_${axis}_start`, 1], [`grid_${axis}_end`, -1]]);
}
add("sticky", [["position", "sticky"]]);
for (const edge of edges) add(`sticky_${edge}`, [["position", "sticky"], [edge, arg()]], valueParam);
add("snap_stop_always", [["scroll_snap_stop", "always"]]);
add("inset_0", edges.map((edge) => [edge, 0]));
for (const suffix of ["", "_start", "_middle"]) add(`text_ellipsis${suffix}`, [["text_overflow", `ellipsis${suffix.replace("_", "-")}`]]);
for (const axis of ["x", "y", ""]) {
  add(`overflow_${axis ? `${axis}_` : ""}scroll`, [["overflow_x", axis === "y" ? "hidden" : "scroll"], ["overflow_y", axis === "x" ? "hidden" : "scroll"]]);
}
add("overflow_hidden", [["overflow_x", "hidden"], ["overflow_y", "hidden"]]);
add("user_select_text", [["user_select", "text"]]);
add("user_select_none", [["user_select", "none"]]);
add("selectable", [["user_select", "text"]]);
const fieldPaths: Record<string, string> = {
  "layout.display": "display", "layout.flex_direction": "flex_direction", "layout.flex_wrap": "flex_wrap",
  "layout.flex_grow": "flex_grow", "layout.flex_shrink": "flex_shrink", "layout.flex_basis": "flex_basis",
  "layout.grid_auto_flow": "grid_auto_flow", "layout.align_items": "align_items", "layout.align_self": "align_self",
  "layout.justify_content": "justify_content", "layout.align_content": "align_content", "visibility": "visibility",
  "layout.position": "position",
};
const css = (variant: string) => ({ NoWrap: "nowrap", RowDense: "row dense", ColumnDense: "column dense" })[variant] ?? variant.replace(/([a-z])([A-Z])/g, "$1-$2").replaceAll("_", "-").toLowerCase();
function constant(source: string): Value {
  const value = source.trim().replaceAll("SPACING_UNIT", String(spacing)).replaceAll("MAX_CORNER_RADIUS", String(maxRadius));
  if (/^[\d.]+(?:\s*\*\s*[\d.]+)?$/.test(value)) return value.split("*").reduce((result, part) => result * Number(part.trim()), 1);
  const weight = /Weight::(NORMAL|MEDIUM|SEMIBOLD|BOLD)/.exec(value)?.[1];
  if (weight) return { NORMAL: 400, MEDIUM: 500, SEMIBOLD: 600, BOLD: 700 }[weight]!;
  if (value === "None") return "normal";
  if (value === "Dimension::auto()") return "auto";
  const length = /Dimension::length\(([\d.]+)\)/.exec(value)?.[1];
  if (length) return Number(length);
  const variant = /(?:\w+::)(\w+)\)?$/.exec(value)?.[1];
  if (variant) return css(variant);
  throw new Error(`Untranslated Rust helper value: ${source}`);
}
const terminalAliases: Record<string, string> = { text_size: "font_size", rounded: "border_radius", gap: "gap" };
function resolveHelper(name: string, pending = new Set<string>()): Helper {
  const ready = helpers.get(name);
  if (ready) return ready;
  const method = methods.get(name)!;
  if (!method || pending.has(name)) throw new Error(`Untranslated Rust layout helper: ${name}`);
  pending.add(name);
  const body = method.body.replace(/\/\/[^\n]*/g, "").trim();
  const ops: Op[] = [];
  if (/^self\./.test(body) && !body.includes(";")) {
    let rest = body.slice(4);
    while (rest.trim()) {
      const call = /^\s*\.(\w+)\(([^()]*)\)/.exec(rest);
      if (!call) throw new Error(`Untranslated Rust helper chain: ${name}: ${rest}`);
      const target = call[1]!;
      if (call[2]!.trim()) {
        const value = constant(call[2]!);
        const fields = aliases[target] ?? [terminalAliases[target] ?? target];
        for (const field of fields) ops.push([field, value]);
        if (target === "rounded") for (const corner of ["top_left", "top_right", "bottom_left", "bottom_right"]) ops.push([`border_${corner}_radius`, null]);
      } else ops.push(...resolveHelper(target, pending).ops);
      rest = rest.slice(call[0].length);
    }
  } else {
    const assignments = [...body.matchAll(/self\.([\w.]+)\s*=\s*([^;]+);/g)];
    if (!assignments.length || assignments.map((assignment) => assignment[0]).join(" ").replace(/\s+/g, " ") !== body.replace(/\s+self$/, "").replace(/\s+/g, " ")) throw new Error(`Untranslated Rust helper body: ${name}`);
    for (const assignment of assignments) {
      const field = fieldPaths[assignment[1]!];
      if (!field) throw new Error(`Untranslated Rust helper field: ${name}: ${assignment[1]}`);
      ops.push([field, constant(assignment[2]!)]);
    }
  }
  const result = { name, parameters: [], ops };
  helpers.set(name, result);
  return result;
}
// These are constructors, retained-object integrations, or canonical value setters,
// rather than conveniences. The Go SDK exposes the value setters under their full names.
const canonical = new Set(["id", "child", "children", "when", "grid_template_columns", "grid_template_rows", "flex_basis", "flex_grow", "flex_shrink", "aspect_ratio", "gap", "margin", "padding", "direction", "scroll_snap_x", "scroll_snap_y"]);
for (const method of methods.values()) {
  if (canonical.has(method.name) || helpers.has(method.name)) continue;
  resolveHelper(method.name);
}
const goName = (name: string) => name.split("_").map((part) => part[0]!.toUpperCase() + part.slice(1)).join("");
function literal(value: Value): string {
  if (value === null) return "clearStyleValue{}";
  if (typeof value !== "object") return JSON.stringify(value);
  if (!("kind" in value)) return value.arg;
  if (value.kind === "percent") return `layoutFraction(${value.arg})`;
  return `layoutGridTracks(${value.arg}, ${JSON.stringify(value.min)}, ${JSON.stringify(value.max)})`;
}
function generateGoHelpers() {
  let go = "// Code generated by bun scripts/generate-style-helpers.ts; DO NOT EDIT.\n\npackage ui\n\n";
  for (const helper of [...helpers.values()].sort((a, b) => a.name.localeCompare(b.name))) {
    const name = goName(helper.name);
    const parameters = helper.parameters.map((param) => `${param.name} ${{ value: "any", number: "float64", count: "int", string: "string" }[param.type]}`).join(", ");
    const options = helper.ops.map(([field, value]) => `style${goName(field)}(${literal(value)})`).join(", ");
    if (name !== "Flex" && name !== "FlexWrap") {
      go += `// ${name} matches Rust's ${helper.name} helper.\nfunc (element *Element) ${name}(${parameters}) *Element { return element.configureStyles([]string{${helper.ops.map(([field]) => JSON.stringify(goName(field))).join(", ")}}, ${options}) }\n\n`;
      go += `// ${name} matches Rust's ${helper.name} helper.\nfunc (style StyleBuilder) ${name}(${parameters}) StyleBuilder { return style.configure(${options}) }\n\n`;
    }
  }
  const result = Bun.spawnSync(["gofmt"], { stdin: Buffer.from(go), stdout: "pipe", stderr: "pipe" });
  if (result.exitCode) throw new Error(result.stderr.toString());
  const path = "go/ui/layout_helpers_generated.go";
  if (check) {
    if (read(path) !== result.stdout.toString()) throw new Error(`${path} is stale; run bun scripts/generate-style-helpers.ts`);
  } else writeFileSync(resolve(root, path), result.stdout);
}

const tsName = (name: string) => name.replace(/_([a-z0-9])/g, (_, letter: string) => letter.toUpperCase());
// `sticky` already controls popover collision behavior in the TypeScript component API.
const tsHelperName = (name: string) => name === "sticky" ? "position-sticky" : name.replaceAll("_", "-");
// Rust enum spellings are not the hosted CSS cursor vocabulary. Read the host's own mapping.
const cursorNames = new Map<string, string>([[css("Arrow"), "default"]]);
const hostCursor = read("crates/quickgui-host/src/view.rs").split("fn cursor(value: &str)")[1]!.split("pub(super) fn")[0]!;
for (const match of hostCursor.matchAll(/"([a-z-]+)"(?:\s*\|\s*"[a-z-]+")*\s*=>\s*CursorStyle::(\w+)/g)) cursorNames.set(css(match[2]!), match[1]!);
function tsValue(field: string, value: Value): Value {
  if (field !== "cursor" || typeof value !== "string") return value;
  const cursor = cursorNames.get(value);
  if (!cursor) throw new Error(`No hosted cursor spelling for ${value}`);
  return cursor;
}
function tsLiteral(value: Value, helper: Helper): string {
  if (value === null || typeof value !== "object") return JSON.stringify(value);
  const input = helper.parameters.length === 1 ? "value" : `(value as readonly unknown[])[${helper.parameters.findIndex((parameter) => parameter.name === value.arg)}]`;
  if (!("kind" in value)) return input;
  if (value.kind === "percent") return `fraction(${input})`;
  return `gridTracks(${input}, ${JSON.stringify(value.min)}, ${JSON.stringify(value.max)})`;
}
function generateTypeScriptHelpers() {
  const sorted = [...helpers.values()].sort((a, b) => a.name.localeCompare(b.name));
  const presets = Object.fromEntries(sorted.filter((helper) => helper.name.startsWith("rounded_") && helper.parameters.length === 0).map((helper) => [helper.name.slice(8), helper.ops[0]![1]]));
  const renderer = read("packages/solid/src/index.ts").split("const properties:")[1]!.split("/** Background properties")[0]!;
  for (const helper of sorted) {
    for (const [field] of helper.ops) {
      if (!new RegExp(`\\b${tsName(field)}:`).test(renderer)) throw new Error(`TypeScript cannot project Rust helper ${helper.name}: missing ${tsName(field)}`);
    }
  }
  let source = `// Code generated by bun scripts/generate-style-helpers.ts; DO NOT EDIT.\n\n`;
  source += `export const roundedPresets = ${JSON.stringify(presets)} as const;\nexport type RoundedPreset = keyof typeof roundedPresets;\n\n`;
  source += `/** Rust Element conveniences in kebab-case, usable as JSX props or inside a style object. */\nexport interface StyleHelpers {\n`;
  for (const helper of sorted) {
    let type = helper.parameters.length === 0 ? "boolean" : helper.parameters.length === 2 ? "readonly [width: number | string, height: number | string]" : helper.parameters[0]!.type === "value" ? "number | string" : helper.parameters[0]!.type === "string" ? '"none" | "start" | "center" | "end"' : "number";
    if (helper.name.startsWith("rounded") && helper.parameters.length) type = "number | RoundedPreset";
    if (helper.name === "flex") type = "boolean | number | string";
    if (helper.name === "flex_wrap") type = 'boolean | "nowrap" | "wrap" | "wrap-reverse"';
    source += `  /** Matches Rust's ${helper.name}${helper.name === "size" ? "; pass [width, height]" : ""}. */\n  ${JSON.stringify(tsHelperName(helper.name))}?: ${type}${helper.parameters.length ? " | false" : ""} | null | undefined;\n`;
  }
  source += `}\n\nexport type HelperDeclaration = Record<string, unknown>;\nexport interface StyleHelperDefinition {\n  parameters: number;\n  fields: readonly string[];\n  resolve: (value: unknown) => HelperDeclaration;\n}\n\n`;
  source += `export const styleHelpers: Record<string, StyleHelperDefinition> = {\n`;
  for (const helper of sorted) {
    const fields = helper.ops.map(([field]) => tsName(field));
    source += `  ${JSON.stringify(tsHelperName(helper.name))}: { parameters: ${helper.parameters.length}, fields: ${JSON.stringify(fields)}, resolve: (value) => ({ ${helper.ops.map(([field, value]) => `${tsName(field)}: ${tsLiteral(tsValue(field, value), helper)}`).join(", ")} }) },\n`;
  }
  source += `};\n\nexport const helperProperties = new Set(Object.values(styleHelpers).flatMap((helper) => helper.fields));\n\n`;
  source += `function fraction(value: unknown): string {\n  const number = Number(value);\n  return \`\${Number.isFinite(number) && number >= 0 ? number * 100 : 0}%\`;\n}\n\n`;
  source += `function gridTracks(value: unknown, minimum: string, maximum: string): string {\n  const count = Number(value);\n  if (!Number.isFinite(count) || count < 1) return "none";\n  return \`repeat(\${Math.min(Math.trunc(count), 1024)}, minmax(\${minimum}, \${maximum}))\`;\n}\n`;
  const path = "packages/solid/src/style-helpers.generated.ts";
  const result = Bun.spawnSync(["bunx", "oxfmt", "--stdin-filepath", path], { cwd: root, stdin: Buffer.from(source), stdout: "pipe", stderr: "pipe" });
  if (result.exitCode) throw new Error(result.stderr.toString());
  if (check) {
    if (read(path) !== result.stdout.toString()) throw new Error(`${path} is stale; run bun scripts/generate-style-helpers.ts`);
  } else writeFileSync(resolve(root, path), result.stdout);
}
if (import.meta.main) {
  generateGoHelpers();
  generateTypeScriptHelpers();
  console.log(`${helpers.size} Rust layout helpers checked for Go and TypeScript`);
}
