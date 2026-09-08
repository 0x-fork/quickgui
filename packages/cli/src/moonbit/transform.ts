import type { Node, Parser } from "web-tree-sitter";
import generatedAPI from "./api.generated.json";
import { children, parseMoonbit, unwrap } from "./parser.ts";
import { CliError } from "../error.ts";

export type ViewKind = "element" | "style" | "unknown";
type Kind = ViewKind;
type Environment = Map<string, Kind>;
type Piece = { text: string; source: number; exact: boolean };
type Pieces = Piece[];
type Argument = { node: Node; value: Node; name?: string; optional: boolean; forwarded: boolean };
type Call = { node: Node; receiver: Node; name: string; args: Argument[] };
type Constructor = { content?: string };
const api = generatedAPI as {
  constructors: Record<string, Constructor>;
  methods: Record<string, { code: number; interaction: boolean; valueType: string }>;
  styleMethods: string[];
  elementFunctions: string[];
  elementMethods: string[];
};
const styleMethods = new Set([...api.styleMethods, "style"]);
const elementMethods = new Set(api.elementMethods);
const lambdas = new Set([
  "arrow_function_expression",
  "anonymous_lambda_expression",
  "anonymous_matrix_lambda_expression",
]);

export interface ViewSourceMap {
  /** Generated offset, original offset, length of an unchanged source slice (zero for generated text). */
  spans: [number, number, number][];
  generatedLines: number[];
  originalLines: number[];
}
export interface TransformedMoonbit {
  code: string;
  map: ViewSourceMap;
  bindings: number;
}

function lineStarts(source: string): number[] {
  const starts = [0];
  for (let i = 0; i < source.length; i++) if (source[i] === "\n") starts.push(i + 1);
  return starts;
}
export function identitySourceMap(source: string): ViewSourceMap {
  return {
    spans: [[0, 0, source.length]],
    originalLines: lineStarts(source),
    generatedLines: lineStarts(source),
  };
}
function typeKind(node: Node | undefined, aliases: string[]): Kind {
  const type = node?.text.replace(/^(?:->|:)\s*/, "").trim();
  return aliases.some((alias) => type === `@${alias}.Element`)
    ? "element"
    : aliases.some((alias) => type === `@${alias}.Style`)
      ? "style"
      : "unknown";
}
export function viewFunctions(
  parser: Parser,
  source: string,
  filename: string,
  aliases: string[],
): Record<string, Kind> {
  const tree = parseMoonbit(parser, source, filename);
  try {
    return Object.fromEntries(
      children(tree.rootNode)
        .filter((node) => node.type === "function_definition")
        .flatMap((node) => {
          const fields = children(node);
          const name = fields.find((child) => child.type === "function_identifier")?.text;
          return name
            ? [
                [
                  name,
                  typeKind(
                    fields.find((child) => child.type === "return_type"),
                    aliases,
                  ),
                ],
              ]
            : [];
        }),
    );
  } finally {
    tree.delete();
  }
}
export function originalPosition(map: ViewSourceMap, line: number, column: number) {
  const offset = (map.generatedLines[line - 1] ?? 0) + column - 1;
  let lo = 0,
    hi = map.spans.length;
  while (lo + 1 < hi) {
    const mid = (lo + hi) >>> 1;
    if (map.spans[mid]![0] <= offset) lo = mid;
    else hi = mid;
  }
  const span = map.spans[lo] ?? [0, 0, 0];
  const original = span[1] + Math.max(0, Math.min(offset - span[0], span[2]));
  lo = 0;
  hi = map.originalLines.length;
  while (lo + 1 < hi) {
    const mid = (lo + hi) >>> 1;
    if (map.originalLines[mid]! <= original) lo = mid;
    else hi = mid;
  }
  return { line: lo + 1, column: original - map.originalLines[lo]! + 1 };
}

function argumentsOf(node: Node): Argument[] {
  const args =
    node.type === "apply_expression"
      ? children(node).find((child) => child.type === "arguments")
      : node;
  if (!args) return [];
  return children(args)
    .filter((child) => child.type === "argument")
    .map((argument) => {
      const inner = children(argument)[0]!;
      if (["labelled_argument", "optional_argument"].includes(inner.type)) {
        const fields = children(inner);
        return {
          node: argument,
          name: fields[0]!.text.replace(/[?~]$/, ""),
          value: fields[1]!,
          optional: inner.type === "optional_argument",
          forwarded: false,
        };
      }
      if (["forwarded_labelled_argument", "forwarded_optional_argument"].includes(inner.type)) {
        return {
          node: argument,
          name: inner.text.replace(/[?~]$/, ""),
          value: inner,
          optional: inner.type === "forwarded_optional_argument",
          forwarded: true,
        };
      }
      return { node: argument, value: inner, optional: false, forwarded: false };
    });
}

function methodCall(node: Node): Call | undefined {
  if (node.type !== "dot_apply_expression") return;
  const fields = children(node);
  const name = fields.find((child) => child.type === "dot_identifier")?.text.slice(1);
  return name ? { node, receiver: fields[0]!, name, args: argumentsOf(node) } : undefined;
}

/** Resolve the actual SDK import, including renamed aliases; unrelated @ui packages are untouched. */
export function uiAliases(parser: Parser, manifest: string, filename = "moon.pkg"): string[] {
  if (filename.endsWith(".json")) {
    const imports = JSON.parse(manifest).import ?? [];
    return imports.flatMap((item: string | { path: string; alias?: string }) => {
      const path = typeof item === "string" ? item : item.path;
      return path === "egoist/quickgui/ui"
        ? [typeof item === "string" ? "ui" : (item.alias ?? "ui")]
        : [];
    });
  }
  const tree = parseMoonbit(parser, manifest, filename);
  try {
    return tree.rootNode.descendantsOfType("import_item").flatMap((item) => {
      const path = item.childForFieldName("path");
      if (!path || JSON.parse(path.text) !== "egoist/quickgui/ui") return [];
      const alias = item.childForFieldName("alias");
      return [alias?.text.replace(/^@/, "") ?? "ui"];
    });
  } finally {
    tree.delete();
  }
}

/** Transform only native view sites. Ordinary MoonBit setup, callbacks, and explicit bindings keep their semantics. */
export function transformMoonbit(
  parser: Parser,
  source: string,
  filename: string,
  aliases: string[] = ["ui"],
  packageFunctions: Record<string, Kind> = {},
): TransformedMoonbit {
  if (!aliases.length) return { code: source, bindings: 0, map: identitySourceMap(source) };
  const tree = parseMoonbit(parser, source, filename);
  const imports = new Set(aliases.map((alias) => `@${alias}`));
  const module = `@${aliases[0]}`;
  let bindingCount = 0;
  let serial = 0;
  const fresh = () => {
    let name: string;
    do {
      name = `__quickgui_branch_${serial++}`;
    } while (source.includes(name));
    return name;
  };
  const copy = (start: number, end: number): Piece => ({
    text: source.slice(start, end),
    source: start,
    exact: true,
  });
  const inject = (text: string, at: Node | number): Piece => ({
    text,
    source: typeof at === "number" ? at : at.startIndex,
    exact: false,
  });
  const join = (items: Pieces[], separator: string, at: Node): Pieces =>
    items.flatMap((item, i) => (i ? [inject(separator, at), ...item] : item));
  const qualified = (node: Node) => {
    if (node.type !== "apply_expression") return;
    const callee = children(node)[0]!;
    const match = /^(@\w+)\.(\w+)$/.exec(callee.text);
    if (match && imports.has(match[1]!)) return { module: match[1]!, name: match[2]! };
  };
  const functions = new Map(Object.entries(packageFunctions));
  for (const node of children(tree.rootNode)) {
    if (node.type !== "function_definition") continue;
    const fields = children(node);
    const name = fields.find((child) => child.type === "function_identifier")?.text;
    if (name)
      functions.set(
        name,
        typeKind(
          fields.find((child) => child.type === "return_type"),
          aliases,
        ),
      );
  }
  const kind = (input: Node, env: Environment): Kind => {
    const node = unwrap(input);
    const call = qualified(node);
    if (call)
      return call.name === "style"
        ? "style"
        : api.elementFunctions.includes(call.name)
          ? "element"
          : "unknown";
    if (node.type === "qualified_identifier") return env.get(node.text) ?? "unknown";
    const method = methodCall(node);
    if (method) {
      const receiver = kind(method.receiver, env);
      if (receiver === "element" && elementMethods.has(method.name)) return "element";
      if (receiver === "style" && !["json", "to_json"].includes(method.name)) return "style";
    }
    if (node.type === "apply_expression")
      return (
        env.get(children(node)[0]!.text) ?? functions.get(children(node)[0]!.text) ?? "unknown"
      );
    if (node.type === "block_expression") {
      const body = children(node);
      return body.length ? kind(body.at(-1)!, env) : "unknown";
    }
    return "unknown";
  };
  const dynamic = (input: Node, callbacks = false): boolean => {
    const node = unwrap(input);
    if (lambdas.has(node.type) && !callbacks) return false;
    const call = qualified(node);
    if (call) {
      if (["style", "rgb8", "rgba8"].includes(call.name))
        return argumentsOf(node).some((arg) => dynamic(arg.value, callbacks));
      if (api.elementFunctions.includes(call.name)) return false;
    }
    const method = methodCall(node);
    if (
      method &&
      (api.styleMethods.includes(method.name) || ["merge", "when"].includes(method.name))
    ) {
      // Only style construction is known pure; ordinary functions with the same method name still run in a binding.
      const base = (() => {
        let value = method.receiver;
        while (methodCall(value)) value = methodCall(value)!.receiver;
        return qualified(value);
      })();
      if (base?.name === "style" || callbacks)
        return (
          dynamic(method.receiver, callbacks) ||
          method.args.some((arg) => dynamic(arg.value, callbacks))
        );
    }
    if (
      node.type === "apply_expression" ||
      node.type === "dot_apply_expression" ||
      node.type === "dot_dot_apply_expression"
    )
      return true;
    return children(node).some((child) => dynamic(child, callbacks));
  };
  const normal = (node: Node, env: Environment, existing = true): Pieces => {
    let cursor = node.startIndex;
    const output: Pieces = [];
    for (const child of children(node)) {
      output.push(copy(cursor, child.startIndex), ...render(child, env, existing));
      cursor = child.endIndex;
    }
    output.push(copy(cursor, node.endIndex));
    return output;
  };
  const argValue = (arg: Argument, env: Environment, existing = true): Pieces =>
    arg.forwarded ? [inject(arg.name!, arg.node)] : render(arg.value, env, existing);
  const closure = (node: Node, env: Environment, valueType?: string): Pieces => [
    inject(
      valueType && /^(String|Bool|Json|Float|Double|Int)$/.test(valueType)
        ? `fn () -> ${valueType} { `
        : "() => { ",
      node,
    ),
    ...render(node, env),
    inject(" }", node.endIndex),
  ];

  function childSuffix(value: Node, env: Environment, array = false): Pieces {
    const node = unwrap(value);
    if (node.type === "if_expression" && dynamic(children(node)[0]!)) {
      // Flatten else-if into a selector so each predicate is evaluated once, and
      // unchanged branch selection does not recreate any of its nodes.
      const cases: Node[] = [];
      const selector = (branch: Node): Pieces | undefined => {
        const fields = children(branch);
        if (branch.type !== "if_expression") {
          const index = cases.push(branch) - 1;
          return [inject(String(index), branch)];
        }
        const condition = fields[0]!;
        if (condition.type === "is_expression") return;
        const body = fields[1]!;
        const otherwise = fields.find((child) => child.type === "else_clause");
        if (!otherwise) return;
        const index = cases.push(body) - 1;
        const rest = selector(children(otherwise)[0]!);
        if (!rest) return;
        return [
          inject("if ", condition),
          ...render(condition, env),
          inject(` { ${index} } else { `, body),
          ...rest,
          inject(" }", otherwise.endIndex),
        ];
      };
      const select = selector(node);
      if (select) {
        bindingCount++;
        const name = fresh();
        return [
          inject(".children_switch(() => { ", node),
          ...select,
          inject(` }, ${name} => match ${name} { `, node),
          ...cases.flatMap((body, index) => [
            inject(`${index} => ${array ? "" : "["}`, body),
            ...render(body, env),
            inject(`${array ? "" : "]"}; `, body.endIndex),
          ]),
          inject("_ => [] })", node.endIndex),
        ];
      }
    }
    if ((node.type === "if_expression" || node.type === "match_expression") && dynamic(node)) {
      bindingCount++;
      return [
        inject(`.children_dynamic(() => ${array ? "" : "["}`, node),
        ...render(node, env),
        inject(`${array ? "" : "]"})`, node.endIndex),
      ];
    }
    if (array && dynamic(node)) {
      bindingCount++;
      return [inject(".bind_content(", node), ...closure(node, env), inject(")", node.endIndex)];
    }
    if (array)
      return [inject(".children(", node), ...render(node, env), inject(")", node.endIndex)];
    return [inject(".child(", node), ...render(node, env), inject(")", node.endIndex)];
  }
  function content(value: Node, env: Environment): { empty?: Pieces; suffix: Pieces } | undefined {
    const node = unwrap(value);
    if (node.type === "array_expression") {
      const entries = children(node);
      if (entries[0]?.type === "list_comprehension_expression") {
        if (!dynamic(node)) return;
        bindingCount++;
        return {
          suffix: [
            inject(".bind_content(", node),
            ...closure(node, env),
            inject(")", node.endIndex),
          ],
        };
      }
      const spread = (entry: Node) => entry.previousSibling?.text === "..";
      if (
        !entries.some(
          (entry) =>
            ["if_expression", "match_expression"].includes(unwrap(entry).type) || spread(entry),
        )
      )
        return;
      return {
        suffix: entries.flatMap((entry) => {
          if (!spread(entry)) return childSuffix(entry, env);
          return childSuffix(entry, env, true);
        }),
      };
    }
    if (dynamic(node) && kind(node, env) !== "element") {
      bindingCount++;
      return {
        suffix: [inject(".bind_content(", node), ...closure(node, env), inject(")", node.endIndex)],
      };
    }
  }

  function elementChain(node: Node, env: Environment, existing: boolean): Pieces {
    const calls: Call[] = [];
    let base = node;
    while (methodCall(base)) {
      const method = methodCall(base)!;
      // A slot creates a new node. Properties before it belong to its receiver.
      if (["slot", "resizable_panel"].includes(method.name)) break;
      calls.unshift(method);
      base = method.receiver;
    }
    const call = qualified(base);
    const ctor = call && api.constructors[call.name];
    const args = ctor ? argumentsOf(base) : [];
    const named = args.find((arg) => arg.name);
    if (named) {
      const { row, column } = named.node.startPosition;
      throw new CliError(
        `${filename}:${row + 1}:${column + 1}: View constructors accept only children or content. Use .${named.name}(...) after the constructor.`,
      );
    }
    const canBind = existing || !!call || base.type === "apply_expression";
    const styles = calls.filter((method) => styleMethods.has(method.name));
    const bindStyles =
      canBind && styles.some((method) => method.args.some((arg) => dynamic(arg.value, true)));
    let stylePieces: Pieces = [];
    if (bindStyles) {
      bindingCount++;
      stylePieces = [inject(`.bind_style(() => ${module}.style()`, base)];
      for (const method of styles) {
        stylePieces.push(
          inject(`.${method.name === "style" ? "merge" : method.name}(`, method.node),
          ...join(
            method.args.map((arg) => argValue(arg, env)),
            ", ",
            method.node,
          ),
          inject(")", method.node.endIndex),
        );
      }
      stylePieces.push(inject(")", node.endIndex));
    }
    let output: Pieces;
    if (ctor && call) {
      const kept: Pieces[] = [];
      const suffix: Pieces = [];
      let position = 0;
      for (const arg of args) {
        if (!arg.name && position++ === 0 && ctor.content) {
          const child = ["array", "content"].includes(ctor.content)
            ? content(arg.value, env)
            : undefined;
          if (child) {
            kept.push([
              inject(
                ctor.content === "content" ? `([] : Array[${module}.Element])` : "[]",
                arg.node,
              ),
            ]);
            suffix.push(...child.suffix);
          } else if (["text", "source"].includes(ctor.content) && dynamic(arg.value)) {
            bindingCount++;
            kept.push([inject('""', arg.node)]);
            if (ctor.content === "text") {
              // One view and one retained label, matching the ordinary text constructor.
              output = [
                inject(`${call.module}.div([]).bind_content(`, base),
                ...closure(arg.value, env),
                inject(")", base.endIndex),
              ];
              return finish(output);
            }
            suffix.push(
              inject(".bind_value(", arg.node),
              ...closure(arg.value, env),
              inject(")", arg.node.endIndex),
            );
          } else kept.push(normal(arg.node, env));
        } else kept.push(normal(arg.node, env));
      }
      output = [
        inject(`${call.module}.${call.name}(`, base),
        ...join(kept, ", ", base),
        inject(")", base.endIndex),
        ...suffix,
      ];
    } else output = normal(base, env, existing);
    return finish(output);

    function finish(result: Pieces): Pieces {
      for (const method of calls) {
        if (bindStyles && styleMethods.has(method.name)) continue;
        const property = api.methods[method.name];
        if (
          canBind &&
          method.name === "label" &&
          method.args.length === 1 &&
          dynamic(method.args[0]!.value)
        ) {
          bindingCount++;
          result.push(
            inject(".bind_label(", method.node),
            ...closure(method.args[0]!.value, env),
            inject(")", method.node.endIndex),
          );
        } else if (
          canBind &&
          method.name === "property" &&
          method.args.length === 2 &&
          dynamic(method.args[1]!.value)
        ) {
          bindingCount++;
          result.push(
            inject(".bind(", method.node),
            ...argValue(method.args[0]!, env),
            inject(", ", method.node),
            ...closure(method.args[1]!.value, env),
            inject(")", method.node.endIndex),
          );
        } else if (
          canBind &&
          property &&
          method.args.length === 1 &&
          (dynamic(method.args[0]!.value, property.interaction) ||
            (property.interaction && !lambdas.has(unwrap(method.args[0]!.value).type)))
        ) {
          bindingCount++;
          const parameter = property.interaction ? fresh() : "";
          result.push(
            inject(
              `.${property.interaction ? "bind_interaction" : "bind_compiled"}(${property.code}, `,
              method.node,
            ),
            ...(property.interaction
              ? lambdas.has(unwrap(method.args[0]!.value).type)
                ? argValue(method.args[0]!, env)
                : [
                    inject(`${parameter} => (`, method.node),
                    ...argValue(method.args[0]!, env),
                    inject(
                      ` : (${module}.Style) -> ${module}.Style)(${parameter})`,
                      method.node.endIndex,
                    ),
                  ]
              : closure(method.args[0]!.value, env, property.valueType)),
            inject(")", method.node.endIndex),
          );
        } else if (canBind && method.name === "children" && method.args.length === 1) {
          const value = content(method.args[0]!.value, env);
          if (value) result.push(...value.suffix);
          else
            result.push(
              inject(".children(", method.node),
              ...render(method.args[0]!.value, env),
              inject(")", method.node.endIndex),
            );
        } else if (canBind && method.name === "child" && method.args.length === 1) {
          result.push(...childSuffix(method.args[0]!.value, env));
        } else {
          result.push(
            copy(method.receiver.endIndex, method.args[0]?.node.startIndex ?? method.node.endIndex),
          );
          if (method.args.length) {
            let cursor = method.args[0]!.node.startIndex;
            for (const arg of method.args) {
              result.push(
                copy(cursor, arg.node.startIndex),
                ...normal(arg.node, env, method.name.startsWith("on_") ? false : existing),
              );
              cursor = arg.node.endIndex;
            }
            result.push(copy(cursor, method.node.endIndex));
          }
        }
      }
      result.push(...stylePieces);
      return result;
    }
  }

  function render(node: Node, env: Environment, existing = true): Pieces {
    // MoonBit's #|/$| literals consume the rest of the physical line. Generated
    // closing delimiters must start on the next line, including inside a thunk.
    if (node.type === "multiline_string_literal")
      return [copy(node.startIndex, node.endIndex), inject("\n", node.endIndex)];
    if (node.type === "function_definition" || lambdas.has(node.type)) {
      const local = new Map(env);
      const params = children(node).find((child) => child.type === "parameters");
      if (params)
        for (const parameter of children(params)) {
          const definition = children(parameter)[0] ?? parameter;
          const fields = children(definition);
          const name = fields[0]?.text.replace(/[?~]$/, "");
          if (name)
            local.set(
              name,
              typeKind(
                fields.find((field) => field.type === "type_annotation"),
                aliases,
              ),
            );
        }
      else if (lambdas.has(node.type)) {
        const first = children(node)[0];
        if (first?.type === "lowercase_identifier") local.set(first.text, "unknown");
      }
      return normal(node, local, existing);
    }
    if (node.type === "block_expression") return normal(node, new Map(env), existing);
    if (node.type === "let_expression") {
      const fields = children(node);
      const output = normal(node, env, existing);
      const declared = typeKind(
        fields.find((field) => field.type === "type_annotation"),
        aliases,
      );
      const name = fields[0]!;
      if (name.type === "lowercase_identifier")
        env.set(name.text, declared !== "unknown" ? declared : kind(fields.at(-1)!, env));
      else
        for (const identifier of name.descendantsOfType("lowercase_identifier"))
          env.set(identifier.text, "unknown");
      return output;
    }
    if (
      (node.type === "apply_expression" || node.type === "dot_apply_expression") &&
      kind(node, env) === "element"
    )
      return elementChain(node, env, existing);
    return normal(node, env, existing);
  }
  try {
    const pieces = render(tree.rootNode, new Map());
    const spans: ViewSourceMap["spans"] = [];
    let code = "";
    for (const piece of pieces) {
      if (!piece.text) continue;
      spans.push([code.length, piece.source, piece.exact ? piece.text.length : 0]);
      code += piece.text;
    }
    return {
      code,
      bindings: bindingCount,
      map: { spans, originalLines: lineStarts(source), generatedLines: lineStarts(code) },
    };
  } finally {
    tree.delete();
  }
}
