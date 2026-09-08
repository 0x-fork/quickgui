import type { Node, Parser } from "web-tree-sitter";
import { children, parseMoonbit, unwrap } from "../packages/cli/src/moonbit/parser.ts";

/** Use accessors in examples when a local signal does not escape as a Signal value. */
export function signalPairs(parser: Parser, source: string, filename: string): string {
  const tree = parseMoonbit(parser, source, filename);
  const calls = (node: Node) => {
    if (node.type !== "dot_apply_expression") return;
    const fields = children(node);
    return {
      receiver: fields[0]!,
      name: fields.find((child) => child.type === "dot_identifier")!.text.slice(1),
      args: fields.filter((child) => child.type === "argument"),
    };
  };
  const safe = (node: Node, name: string) => {
    let scope = node.parent!;
    while (scope.parent && scope.type !== "block_expression") scope = scope.parent;
    const references = scope
      .descendantsOfType("qualified_identifier")
      .filter((reference) => reference.text === name);
    if (!references.length || source.includes(`set_${name}`)) return false;
    return references.every((reference) => {
      let parent = reference.parent;
      while (parent && ["atomic_expression", "parenthesized_expression"].includes(parent.type))
        parent = parent.parent;
      const call = parent && calls(parent);
      return (
        call && call.receiver.text === name && ["get", "set", "peek", "update"].includes(call.name)
      );
    });
  };
  function normal(node: Node, env: Set<string>, replacements: Map<string, string>): string {
    let output = "",
      cursor = node.startIndex;
    for (const child of children(node)) {
      output += source.slice(cursor, child.startIndex) + render(child, env, replacements);
      cursor = child.endIndex;
    }
    return output + source.slice(cursor, node.endIndex);
  }
  function render(node: Node, env: Set<string>, replacements: Map<string, string>): string {
    if (node.type === "qualified_identifier" && replacements.has(node.text))
      return replacements.get(node.text)!;
    if (node.type === "block_expression") return normal(node, new Set(env), replacements);
    if (node.type === "let_expression") {
      const fields = children(node),
        name = fields[0]!.text,
        value = unwrap(fields.at(-1)!);
      if (
        fields[0]!.type === "lowercase_identifier" &&
        value.type === "apply_expression" &&
        children(value)[0]?.text === "@reactive.signal" &&
        safe(node, name)
      ) {
        env.add(name);
        let scope = node.parent!;
        while (scope.parent && scope.type !== "block_expression") scope = scope.parent;
        const written = scope.descendantsOfType("dot_apply_expression").some((call) => {
          const info = calls(call)!;
          return info.receiver.text === name && ["set", "update"].includes(info.name);
        });
        const annotation = fields.find((field) => field.type === "type_annotation");
        const type = annotation?.text.match(/^:\s*@reactive\.Signal\[([\s\S]+)\]$/)?.[1];
        let initializer = normal(value, env, replacements).replace(
          /^@reactive\.signal/,
          "@ui.create_signal",
        );
        if (type) {
          const argument = children(
            children(value).find((field) => field.type === "arguments")!,
          )[0]!;
          initializer = `@ui.create_signal((${render(argument, env, replacements)} : ${type}))`;
        }
        return `let (${name}, ${written ? `set_${name}` : "_"}) = ${initializer}`;
      }
      const output = normal(node, env, replacements);
      env.delete(name);
      return output;
    }
    const call = calls(node);
    if (call && env.has(call.receiver.text)) {
      const name = call.receiver.text;
      if (call.name === "get") return `${name}()`;
      if (call.name === "peek") return `@ui.untrack(${name})`;
      if (call.name === "set")
        return `set_${name}(${call.args.map((arg) => render(arg, env, replacements)).join(", ")})`;
      if (call.name === "update") {
        let parent = node.parent;
        let event = false;
        while (parent) {
          if (
            (parent.type === "labelled_argument" && parent.text.startsWith("on_")) ||
            calls(parent)?.name.startsWith("on_")
          ) {
            event = true;
            break;
          }
          parent = parent.parent;
        }
        const getter = event ? `${name}()` : `@ui.untrack(${name})`;
        const callback = unwrap(call.args[0]!);
        if (
          callback.type === "arrow_function_expression" &&
          children(callback)[0]?.type === "lowercase_identifier"
        ) {
          const parts = children(callback),
            local = new Map(replacements);
          const body = parts.at(-1)!;
          if (body.type === "block_expression") {
            // Keep callback-local returns and evaluate the old value once.
            local.delete(parts[0]!.text);
            const rendered = render(body, env, local);
            return `set_${name}((() => { let ${parts[0]!.text} = ${getter}\n${rendered.slice(1, -1)}\n})())`;
          }
          local.set(parts[0]!.text, getter);
          return `set_${name}(${render(parts.at(-1)!, env, local)})`;
        }
        return `set_${name}((${render(callback, env, replacements)})(${getter}))`;
      }
    }
    return normal(node, env, replacements);
  }
  try {
    return render(tree.rootNode, new Set(), new Map());
  } finally {
    tree.delete();
  }
}
