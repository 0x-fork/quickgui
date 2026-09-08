import type { Node, Parser } from "web-tree-sitter";
import { children, parseMoonbit, unwrap } from "../packages/cli/src/moonbit/parser.ts";
import api from "../packages/cli/src/moonbit/api.generated.json";

/** Keep generated examples on the constructor/list API, including lazy branches. */
export function childLists(parser: Parser, source: string, filename: string): string {
  const tree = parseMoonbit(parser, source, filename);
  const args = (node: Node) =>
    children(
      node.type === "apply_expression"
        ? children(node).find((child) => child.type === "arguments")!
        : node,
    ).filter((child) => child.type === "argument");
  const method = (node: Node) => {
    if (node.type !== "dot_apply_expression") return;
    const fields = children(node);
    return {
      node,
      base: fields[0]!,
      name: fields.find((child) => child.type === "dot_identifier")!.text.slice(1),
      args: args(node),
    };
  };
  const invoke = (argument: Node): string => {
    const node = unwrap(argument);
    if (["arrow_function_expression", "anonymous_lambda_expression"].includes(node.type)) {
      const body = children(node).at(-1)!;
      return render(
        body.type === "block_expression" && children(body).length === 1 ? children(body)[0]! : body,
      );
    }
    return `(${render(node)})()`;
  };
  const entries = (argument: Node): string[] => {
    const node = unwrap(argument);
    return node.type === "array_expression" &&
      children(node)[0]?.type !== "list_comprehension_expression"
      ? children(node).map(
          (child) => `${child.previousSibling?.text === ".." ? ".." : ""}${render(child)}`,
        )
      : [`..${render(node)}`];
  };
  function normal(node: Node): string {
    let output = "",
      cursor = node.startIndex;
    for (const child of children(node)) {
      output += source.slice(cursor, child.startIndex) + render(child);
      cursor = child.endIndex;
    }
    return output + source.slice(cursor, node.endIndex);
  }
  function render(node: Node): string {
    if (!method(node)) return normal(node);
    const chain = [];
    let base = node;
    while (method(base)) {
      const current = method(base)!;
      chain.unshift(current);
      base = current.base;
    }
    if (!chain.some((call) => call.name === "child" || call.name === "child_if"))
      return normal(node);
    const baseArgs = base.type === "apply_expression" ? args(base) : [];
    const constructor = /^@ui\.(\w+)$/.exec(children(base)[0]?.text ?? "")?.[1];
    const constructorInfo = constructor
      ? (api.constructors as Record<string, { content?: string }>)[constructor]
      : undefined;
    const initial = baseArgs.find(
      (arg) =>
        ![
          "labelled_argument",
          "optional_argument",
          "forwarded_labelled_argument",
          "forwarded_optional_argument",
        ].includes(children(arg)[0]!.type),
    );
    const absorb = constructorInfo?.content === "array" && initial;
    let pending: string[] = [],
      suffix = "",
      output = "",
      first = true;
    function flush() {
      if (first) {
        if (absorb && pending.length) {
          output =
            source.slice(base.startIndex, initial.startIndex) +
            `[${[...entries(initial), ...pending].join(",\n")}]` +
            baseArgs
              .filter((arg) => arg.startIndex > initial.startIndex)
              .map((arg, i, rest) => {
                const previous = i ? rest[i - 1]! : initial;
                return source.slice(previous.endIndex, arg.startIndex) + render(arg);
              })
              .join("") +
            source.slice(baseArgs.at(-1)!.endIndex, base.endIndex);
        } else
          output = render(base) + (pending.length ? `.children([${pending.join(",\n")}])` : "");
        first = false;
      } else if (pending.length) output += `.children([${pending.join(",\n")}])`;
      output += suffix;
      pending = [];
      suffix = "";
    }
    for (const call of chain) {
      if (call.name === "child") pending.push(render(call.args[0]!));
      else if (call.name === "children") pending.push(...entries(call.args[0]!));
      else if (call.name === "child_if") {
        const alternative = call.args.find((arg) => arg.text.startsWith("otherwise="));
        let otherwise = "[]";
        if (alternative) {
          const value = children(children(alternative)[0]!)[1]!;
          if (value.text !== "None") {
            const some = unwrap(value);
            const callback = args(some)[0];
            if (!callback)
              throw new Error(`Unsupported child_if fallback in ${filename}: ${alternative.text}`);
            otherwise = `[${invoke(callback)}]`;
          }
        }
        pending.push(
          `..if ${invoke(call.args[0]!)} { [${invoke(call.args[1]!)}] } else { ${otherwise} }`,
        );
      } else if (
        call.name.startsWith("children_") ||
        [
          "slot",
          "resizable_panel",
          "bind_content",
          "label",
          "bind_label",
          "when_open",
          "system_content",
        ].includes(call.name)
      ) {
        flush();
        output += tail(call);
      } else suffix += tail(call);
    }
    flush();
    return output;
  }
  function tail(call: NonNullable<ReturnType<typeof method>>) {
    let output = "",
      cursor = call.base.endIndex;
    for (const argument of call.args) {
      output += source.slice(cursor, argument.startIndex) + render(argument);
      cursor = argument.endIndex;
    }
    return output + source.slice(cursor, call.node.endIndex);
  }
  try {
    return render(tree.rootNode);
  } finally {
    tree.delete();
  }
}
