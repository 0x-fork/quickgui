import type { Node, Parser } from "web-tree-sitter";
import { children, parseMoonbit, unwrap } from "../packages/cli/src/moonbit/parser.ts";
import api from "../packages/cli/src/moonbit/api.generated.json";

/** Documentation uses the public expression syntax instead of compiler targets. */
export function directBindings(parser: Parser, source: string, filename: string): string {
  const tree = parseMoonbit(parser, source, filename);
  function render(node: Node): string {
    const fields = children(node);
    if (node.type === "dot_apply_expression") {
      const method = fields.find((child) => child.type === "dot_identifier")!.text.slice(1);
      const target = method === "bind_number" ? "value" : method.slice(5);
      const args = fields.filter((child) => child.type === "argument");
      if (
        method.startsWith("bind_") &&
        (target in api.methods || ["style", "label"].includes(target)) &&
        args.length === 1
      ) {
        const read = unwrap(args[0]!);
        let value: string;
        if (["arrow_function_expression", "anonymous_lambda_expression"].includes(read.type)) {
          const body = children(read).at(-1)!;
          value = render(
            body.type === "block_expression" && children(body).length === 1
              ? children(body)[0]!
              : body,
          );
        } else value = `(${render(read)})()`;
        return `${render(fields[0]!)}.${target}(${value})`;
      }
    }
    let output = "",
      cursor = node.startIndex;
    for (const child of fields) {
      output += source.slice(cursor, child.startIndex) + render(child);
      cursor = child.endIndex;
    }
    return output + source.slice(cursor, node.endIndex);
  }
  try {
    return render(tree.rootNode);
  } finally {
    tree.delete();
  }
}
