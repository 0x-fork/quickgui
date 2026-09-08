import type { Node, Parser } from "web-tree-sitter";
import { CliError } from "../error.ts";
import { children, parseMoonbit } from "./parser.ts";
import type { ViewSourceMap } from "./transform.ts";

export interface ComponentParameter {
  name: string;
  type: string;
  lazy: boolean;
  labelled: boolean;
}
export interface Component {
  name: string;
  generated: string;
  public: boolean;
  parameters: ComponentParameter[];
}
type Piece = { text: string; at: number; exact: boolean };
type Scope = Map<string, "prop" | "local">;
const functions = new Set([
  "function_definition",
  "named_lambda_expression",
  "arrow_function_expression",
  "anonymous_lambda_expression",
  "anonymous_matrix_lambda_expression",
]);

function fail(node: Node, filename: string, message: string): never {
  throw new CliError(
    `${filename}:${node.startPosition.row + 1}:${node.startPosition.column + 1}: ${message}`,
  );
}
function definition(node: Node, aliases: string[], filename: string): Component | undefined {
  if (node.type !== "function_definition") return;
  const fields = children(node);
  const name = fields.find((field) => field.type === "function_identifier")!.text;
  const result = fields.find((field) => field.type === "return_type")?.text.replace(/^->\s*/, "");
  if (!aliases.some((alias) => result === `@${alias}.Element`)) return;
  if (name.includes("::")) return;
  const parameters = children(fields.find((field) => field.type === "parameters")!).map(
    (parameter) => {
      const inner = children(parameter)[0]!;
      const parts = children(inner);
      const type = parts.find((part) => part.type === "type_annotation")?.text.replace(/^:\s*/, "");
      if (!type || inner.type === "optional_parameter")
        fail(
          parameter,
          filename,
          "Component parameters need explicit types; optional parameters need a default value.",
        );
      return {
        name: parts[0]!.text.replace(/[?~]$/, ""),
        type,
        lazy: !type.includes("->") && !aliases.some((alias) => type.includes(`@${alias}.Element`)),
        labelled: inner.type !== "positional_parameter",
      };
    },
  );
  if (!parameters.some((param) => param.lazy)) return;
  return {
    name,
    generated: `quickgui_component_${name}`,
    public: fields.some((field) => field.type === "visibility" && field.text === "pub"),
    parameters,
  };
}

export function componentDefinitions(
  parser: Parser,
  source: string,
  filename: string,
  aliases: string[],
): Record<string, Component> {
  const tree = parseMoonbit(parser, source, filename);
  try {
    return Object.fromEntries(
      children(tree.rootNode).flatMap((node) => {
        const value = definition(node, aliases, filename);
        return value ? [[value.name, value]] : [];
      }),
    );
  } finally {
    tree.delete();
  }
}

/** Lower component boundaries; the native-view pass owns all subscriptions. */
export function lowerComponents(
  parser: Parser,
  source: string,
  filename: string,
  aliases: string[],
  registry: Record<string, Component>,
) {
  const tree = parseMoonbit(parser, source, filename);
  const components = { ...registry };
  const declarations = new Map<number, Component>();
  for (const node of children(tree.rootNode)) {
    const value = definition(node, aliases, filename);
    if (value) {
      if (
        children(tree.rootNode).some((other) =>
          children(other).some(
            (field) => field.type === "function_identifier" && field.text === value.generated,
          ),
        )
      )
        fail(node, filename, `Reserved generated component name: ${value.generated}`);
      components[value.name] = value;
      declarations.set(node.id, value);
    }
  }
  const copy = (start: number, end: number): Piece => ({
    text: source.slice(start, end),
    at: start,
    exact: true,
  });
  const add = (text: string, node: Node): Piece => ({ text, at: node.startIndex, exact: false });
  const shadow = (node: Node, scope: Scope) => {
    if (node.type === "lowercase_identifier") scope.set(node.text, "local");
    for (const id of node.descendantsOfType("lowercase_identifier")) scope.set(id.text, "local");
  };
  const normal = (node: Node, scope: Scope): Piece[] => {
    let at = node.startIndex;
    const pieces: Piece[] = [];
    for (const child of children(node)) {
      pieces.push(copy(at, child.startIndex), ...render(child, scope));
      at = child.endIndex;
    }
    return [...pieces, copy(at, node.endIndex)];
  };
  const dynamic = (node: Node, scope: Scope): boolean => {
    if (functions.has(node.type)) return false;
    if (node.type === "qualified_identifier" && scope.get(node.text) === "prop") return true;
    if (
      ["apply_expression", "dot_apply_expression", "dot_dot_apply_expression"].includes(node.type)
    )
      return true;
    return children(node).some((child) => dynamic(child, scope));
  };
  function render(node: Node, scope: Scope): Piece[] {
    const component = declarations.get(node.id);
    if (component) {
      const fields = children(node);
      const body = fields.find((field) => field.type === "block_expression")!;
      const name = fields.find((field) => field.type === "function_identifier")!;
      const params = fields.find((field) => field.type === "parameters")!;
      const local = new Map(scope);
      for (const param of component.parameters)
        local.set(param.name, param.lazy ? "prop" : "local");
      const invoke = component.parameters
        .map(
          (param) =>
            `${param.labelled ? `${param.name}=` : ""}${param.lazy ? `${component.generated}_value(${param.name})` : param.name}`,
        )
        .join(", ");
      const pieces = [
        add('#warnings("-unused_value")\n', node),
        copy(node.startIndex, body.startIndex),
        add(`{ ${component.generated}(${invoke}) }\n\n`, body),
        copy(node.startIndex, name.startIndex),
        add(component.generated, name),
        copy(name.endIndex, params.startIndex),
        add("(", params),
      ];
      children(params).forEach((parameter, index) => {
        const param = component.parameters[index]!;
        if (index) pieces.push(add(", ", parameter));
        if (!param.lazy) {
          pieces.push(copy(parameter.startIndex, parameter.endIndex));
          return;
        }
        const inner = children(children(parameter)[0]!);
        const annotation = inner.find((field) => field.type === "type_annotation")!;
        pieces.push(
          copy(parameter.startIndex, annotation.startIndex),
          add(`: () -> ${param.type}`, annotation),
        );
        const value = inner.at(-1)!;
        if (value !== annotation)
          pieces.push(
            copy(annotation.endIndex, value.startIndex),
            add("() => { ", value),
            ...render(value, new Map(scope)),
            add(" }", value),
          );
      });
      return [
        ...pieces,
        add(")", params),
        copy(params.endIndex, body.startIndex),
        ...normal(body, local),
        add(
          `\n\n${component.public ? "pub " : ""}fn[T] ${component.generated}_value(value : T) -> () -> T { () => value }`,
          node,
        ),
      ];
    }
    if (functions.has(node.type)) {
      if (node.type === "named_lambda_expression") shadow(children(node)[0]!, scope);
      const local = new Map(scope);
      const params = children(node).find((child) => child.type === "parameters");
      if (params)
        for (const param of children(params)) shadow(children(children(param)[0]!)[0]!, local);
      else if (children(node)[0]?.type === "lowercase_identifier")
        shadow(children(node)[0]!, local);
      return normal(node, local);
    }
    if (node.type === "block_expression") return normal(node, new Map(scope));
    if (node.type === "let_expression") {
      const fields = children(node);
      const result = normal(node, scope);
      shadow(fields[0]!, scope);
      return result;
    }
    if (node.type === "case_clause") {
      const local = new Map(scope);
      shadow(children(node)[0]!, local);
      return normal(node, local);
    }
    if (node.type === "for_in_expression") {
      const fields = children(node);
      const body = fields.find((field) => field.type === "block_expression")!;
      const local = new Map(scope);
      for (const field of fields.slice(0, -2)) shadow(field, local);
      return [
        ...normalRange(
          node.startIndex,
          body.startIndex,
          fields.filter((field) => field !== body),
          scope,
        ),
        ...render(body, local),
        copy(body.endIndex, node.endIndex),
      ];
    }
    if (node.type === "qualified_identifier" && scope.get(node.text) === "prop")
      return [copy(node.startIndex, node.endIndex), add("()", node)];
    if (node.type === "labeled_expression_pun" && scope.get(node.text) === "prop")
      return [copy(node.startIndex, node.endIndex), add(`: ${node.text}()`, node)];
    if (["forwarded_labelled_argument", "forwarded_optional_argument"].includes(node.type)) {
      const name = node.text.replace(/[?~]$/, "");
      if (scope.get(name) === "prop")
        return [add(`${name}${node.text.endsWith("?") ? "?" : ""}=${name}()`, node)];
    }
    if (node.type === "apply_expression") {
      const fields = children(node);
      const callee = fields[0]!;
      const target = !scope.has(callee.text) && components[callee.text];
      const args = fields.find((field) => field.type === "arguments");
      if (target && args) {
        let positional = 0;
        const qualified = callee.text.startsWith("@")
          ? callee.text.slice(0, callee.text.indexOf(".") + 1)
          : "";
        const values = children(args).map((argument) => {
          const inner = children(argument)[0]!;
          const named = /labelled|optional/.test(inner.type);
          const forwarded = inner.type.startsWith("forwarded_");
          const name = named
            ? (forwarded ? inner.text : children(inner)[0]!.text).replace(/[?~]$/, "")
            : undefined;
          const param = name
            ? target.parameters.find((param) => param.name === name)
            : target.parameters.filter((param) => !param.labelled)[positional++];
          const value = named && !forwarded ? children(inner)[1]! : inner;
          if (!param?.lazy) return render(argument, scope);
          if (inner.type.includes("optional"))
            fail(
              argument,
              filename,
              "Forward optional component props as a concrete value or supply a default first.",
            );
          const live = forwarded ? scope.get(name!) === "prop" : dynamic(value, scope);
          if (!live)
            return [
              add(`${name ? `${name}=` : ""}${qualified}${target.generated}_value(`, argument),
              ...(forwarded ? [add(name!, inner)] : render(value, scope)),
              add(")", argument),
            ];
          return [
            add(`${name ? `${name}=` : ""}() => (`, argument),
            ...(forwarded
              ? [add(`${name}${scope.get(name!) === "prop" ? "()" : ""}`, inner)]
              : render(value, scope)),
            add(")", argument),
          ];
        });
        return [
          add(`${qualified}${target.generated}(`, callee),
          ...values.flatMap((value, i) => (i ? [add(", ", args), ...value] : value)),
          add(")", args),
        ];
      }
    }
    // Multiline strings consume the remainder of their physical line.
    if (node.type === "multiline_string_literal") return [...normal(node, scope), add("\n", node)];
    return normal(node, scope);
  }
  function normalRange(start: number, end: number, nodes: Node[], scope: Scope): Piece[] {
    let at = start;
    const pieces: Piece[] = [];
    for (const node of nodes) {
      pieces.push(copy(at, node.startIndex), ...render(node, scope));
      at = node.endIndex;
    }
    return [...pieces, copy(at, end)];
  }
  try {
    const spans: ViewSourceMap["spans"] = [];
    let code = "";
    for (const piece of render(tree.rootNode, new Map())) {
      if (!piece.text) continue;
      spans.push([code.length, piece.at, piece.exact ? piece.text.length : 0]);
      code += piece.text;
    }
    return { code, spans };
  } finally {
    tree.delete();
  }
}
