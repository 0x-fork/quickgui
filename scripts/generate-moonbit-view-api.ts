#!/usr/bin/env bun
/** Compiler targets come from the SDK, not a second list of component APIs. */
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { createMoonbitParser, parseMoonbit, children } from "../packages/cli/src/moonbit/parser.ts";

const root = resolve(import.meta.dir, "..");
const parser = await createMoonbitParser();
const constants = Object.fromEntries(
  [
    ...readFileSync(resolve(root, "moonbit/protocol/constants.mbt"), "utf8").matchAll(
      /pub const (\w+) : Int = (\d+)/g,
    ),
  ].map(([, name, value]) => [name, Number(value)]),
);
const functions: Record<
  string,
  { parameters: { name: string; type: string; optional: boolean }[]; result: string; body: string }
> = {};
for (const file of readdirSync(resolve(root, "moonbit/ui")).filter(
  (file) => file.endsWith(".mbt") && !file.endsWith("_test.mbt"),
)) {
  const tree = parseMoonbit(parser, readFileSync(resolve(root, "moonbit/ui", file), "utf8"), file);
  for (const node of children(tree.rootNode).filter(
    (node) => node.type === "function_definition" && node.text.startsWith("pub fn"),
  )) {
    const parts = children(node);
    const name = parts.find((part) => part.type === "function_identifier")!.text;
    const parameters = parts.find((part) => part.type === "parameters")!;
    const result =
      parts.find((part) => part.type === "return_type")?.text.replace(/^->\s*/, "") ?? "Unit";
    functions[name] = {
      result,
      body: parts.find((part) => part.type === "block_expression")?.text ?? "",
      parameters: children(parameters).map((parameter) => {
        const value = children(parameter)[0]!;
        const fields = children(value);
        return {
          name: fields[0]!.text.replace(/[?~]$/, ""),
          type: fields.find((field) => field.type === "type_annotation")!.text.replace(/^:\s*/, ""),
          optional: value.type === "optional_parameter",
        };
      }),
    };
  }
  tree.delete();
}
const methods: Record<string, { code: number; interaction: boolean; valueType: string }> = {};
for (const [name, fn] of Object.entries(functions)) {
  if (!name.startsWith("Element::") || fn.parameters.length !== 2 || fn.result !== "Element")
    continue;
  const method = name.slice("Element::".length);
  const property = /(?:property|bind)\(\s*@protocol\.(\w+)/.exec(fn.body)?.[1];
  if (
    !property ||
    constants[property] === undefined ||
    method.startsWith("bind_") ||
    method.startsWith("on_")
  )
    continue;
  methods[method] = {
    code: constants[property]!,
    interaction: fn.parameters[1]!.type.includes("-> Style"),
    valueType: fn.parameters[1]!.type,
  };
}
const constructors: Record<string, { content?: string | undefined }> = {};
for (const [name, fn] of Object.entries(functions)) {
  if (name.includes("::") || fn.result !== "Element" || !/configure_element\(/.test(fn.body))
    continue;
  const first = fn.parameters[0];
  constructors[name] = {
    ...(first && !first.optional
      ? {
          content:
            first.type === "Array[Element]"
              ? "array"
              : first.type === "C"
                ? "content"
                : first.type === "String"
                  ? name === "text"
                    ? "text"
                    : "source"
                  : undefined,
        }
      : {}),
  };
}
// text_area delegates directly to input; shader delegates its source to the core.
constructors.input = {};
constructors.text_area = {};
constructors.shader = { content: "source" };
constructors.table = { content: "array" };
constructors.swift_ui_quickgui_host_view = { content: "factory" };
const styleMethods = Object.keys(functions)
  .filter(
    (name) =>
      name.startsWith("Style::") &&
      functions[`Element::${name.slice(7)}`] &&
      !["disabled", "value", "placeholder", "multiline", "property"].includes(name.slice(7)),
  )
  .map((name) => name.slice(7))
  .sort();
const api = {
  constructors,
  methods,
  styleMethods,
  elementFunctions: Object.entries(functions)
    .filter(([name, fn]) => !name.includes("::") && fn.result === "Element")
    .map(([name]) => name)
    .sort(),
  elementMethods: Object.entries(functions)
    .filter(([name, fn]) => name.startsWith("Element::") && fn.result === "Element")
    .map(([name]) => name.slice(9))
    .sort(),
};
const output = JSON.stringify(api, null, 2) + "\n";
const filename = resolve(root, "packages/cli/src/moonbit/api.generated.json");
if (process.argv.includes("--check")) {
  if (readFileSync(filename, "utf8") !== output)
    throw new Error("Run bun scripts/generate-moonbit-view-api.ts");
} else writeFileSync(filename, output);
parser.delete();
