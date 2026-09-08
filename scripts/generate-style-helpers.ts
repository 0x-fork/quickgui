#!/usr/bin/env bun
/** Generate both frontends' layout conveniences from the Rust Element API. */
import { existsSync, readFileSync, writeFileSync } from "node:fs";
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
// rather than conveniences. Both SDKs expose the value setters under their full names.
const canonical = new Set(["id", "child", "children", "when", "grid_template_columns", "grid_template_rows", "flex_basis", "flex_grow", "flex_shrink", "aspect_ratio", "gap", "margin", "padding", "direction", "scroll_snap_x", "scroll_snap_y"]);
for (const method of methods.values()) {
  if (canonical.has(method.name) || helpers.has(method.name)) continue;
  resolveHelper(method.name);
}
const goName = (name: string) => name.split("_").map((part) => part[0]!.toUpperCase() + part.slice(1)).join("");
const wire = (name: string) => ({ text_color: "COLOR", transition_property: "TRANSITION_PROPERTIES" })[name] ?? name.toUpperCase();
const styleKey = (name: string) => name.replace(/_([a-z])/g, (_, letter) => letter.toUpperCase());
const constants = read("moonbit/protocol/constants.mbt");
function literal(value: Value, language: "go" | "moonbit"): string {
  if (value === null) return language === "go" ? "clearStyleValue{}" : "@protocol.Clear";
  if (typeof value !== "object") return JSON.stringify(value);
  if (!("kind" in value)) return value.arg;
  if (value.kind === "percent") return language === "go" ? `layoutFraction(${value.arg})` : `layout_fraction(${value.arg})`;
  return language === "go" ? `layoutGridTracks(${value.arg}, ${JSON.stringify(value.min)}, ${JSON.stringify(value.max)})` : `layout_grid_tracks(${value.arg}, ${JSON.stringify(value.min)}, ${JSON.stringify(value.max)})`;
}
let go = "// Code generated by bun scripts/generate-style-helpers.ts; DO NOT EDIT.\n\npackage ui\n\n";
let moon = "// Generated by bun scripts/generate-style-helpers.ts. Do not edit.\n";
for (const helper of [...helpers.values()].sort((a, b) => a.name.localeCompare(b.name))) {
  const name = goName(helper.name);
  const parameters = helper.parameters.map((param) => `${param.name} ${{ value: "any", number: "float64", count: "int", string: "string" }[param.type]}`).join(", ");
  const options = helper.ops.map(([field, value]) => `style${goName(field)}(${literal(value, "go")})`).join(", ");
  if (name !== "Flex" && name !== "FlexWrap") {
    go += `// ${name} matches Rust's ${helper.name} helper.\nfunc (element *Element) ${name}(${parameters}) *Element { return element.configureStyles([]string{${helper.ops.map(([field]) => JSON.stringify(goName(field))).join(", ")}}, ${options}) }\n\n`;
    go += `// ${name} matches Rust's ${helper.name} helper.\nfunc (style StyleBuilder) ${name}(${parameters}) StyleBuilder { return style.configure(${options}) }\n\n`;
  }
  for (const receiver of ["Element", "Style"]) {
    const generic = helper.parameters.some((param) => param.type === "value") ? "[T : IntoValue]" : "";
    const params = helper.parameters.map((param) => `${param.name} : ${{ value: "T", number: "Float", count: "Int", string: "String" }[param.type]}`);
    const calls = helper.ops.map(([field, value]) => {
      const code = wire(field);
      if (!constants.includes(`pub const ${code} :`)) throw new Error(`Missing MoonBit wire field ${field}`);
      return `.property(@protocol.${code}, ${receiver === "Style" ? `${JSON.stringify(styleKey(field))}, ` : ""}${value === null ? "@protocol.Clear" : `IntoValue::into_value(${literal(value, "moonbit")})`})`;
    }).join("");
    moon += `\n///|\n/// Matches Rust's ${helper.name} helper.\npub fn${generic} ${receiver}::${helper.name}(self : ${receiver}${params.length ? `, ${params.join(", ")}` : ""}) -> ${receiver} {\nself${calls}\n}\n`;
  }
}
function emit(path: string, source: string, formatter: string[]) {
  const result = Bun.spawnSync(formatter, { stdin: Buffer.from(source), stdout: "pipe", stderr: "pipe" });
  if (result.exitCode) throw new Error(result.stderr.toString());
  const formatted = result.stdout.toString();
  if (check) {
    if (!existsSync(resolve(root, path)) || read(path) !== formatted) throw new Error(`${path} is stale; run bun scripts/generate-style-helpers.ts`);
  } else writeFileSync(resolve(root, path), formatted);
}
emit("go/ui/layout_helpers_generated.go", go, ["gofmt"]);
emit("moonbit/ui/layout_helpers_generated.mbt", moon, [Bun.which("moonfmt") ?? resolve(root, "target/moonbit-toolchain/bin/moonfmt"), "-"]);
console.log(`${helpers.size} Rust layout helpers checked for Go and MoonBit`);
