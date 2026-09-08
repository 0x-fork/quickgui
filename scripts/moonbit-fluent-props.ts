import type { Node, Parser } from "web-tree-sitter";
import { children, parseMoonbit, unwrap } from "../packages/cli/src/moonbit/parser.ts";
import api from "../packages/cli/src/moonbit/api.generated.json";

/** Keep generated examples on content-only constructors and fluent props. */
export function fluentProps(parser: Parser, source: string, filename: string): string {
  const tree = parseMoonbit(parser, source, filename);
  const constructors = new Set([...Object.keys(api.constructors), "swift_ui_quickgui_host_view"]);
  function render(node: Node): string {
    const fields = children(node);
    if (node.type === "apply_expression") {
      const match = /^@ui\.(\w+)$/.exec(fields[0]?.text ?? "");
      const invoke = (argument: Node): string => {
        const value = unwrap(argument);
        if (["arrow_function_expression", "anonymous_lambda_expression"].includes(value.type)) {
          const body = children(value).at(-1)!;
          if (!body.descendantsOfType("return_expression").length) {
            return render(
              body.type === "block_expression" && children(body).length === 1
                ? children(body)[0]!
                : body,
            );
          }
        }
        return `(${render(value)})()`;
      };
      if (match && ["table", "table_row", "table_cell", "resizable_panel"].includes(match[1]!)) {
        const args = children(fields.find((field) => field.type === "arguments")!);
        const name = match[1]!;
        if (name === "resizable_panel") {
          return `${render(args[0]!)}.resizable_panel(${args.slice(1).map(render).join(", ")})`;
        }
        if (name === "table" && args.length >= 3) {
          const selection = args
            .slice(3)
            .map((argument) => {
              const value = children(unwrap(argument));
              return `.selection_mode(${render(value[1]!)})`;
            })
            .join("");
          return `@ui.table([]).columns(${invoke(args[0]!)}).row_count(${invoke(args[1]!)}).row_height(${render(args[2]!)})${selection}`;
        }
        if (name === "table_row" && unwrap(args[0]!).type !== "array_expression") {
          return `@ui.table_row([]).row_index(${invoke(args[0]!)})`;
        }
        if (name === "table_cell" && unwrap(args[0]!).type !== "array_expression") {
          return `@ui.table_cell([]).column(${render(args[0]!)})`;
        }
      }
      if (match && constructors.has(match[1]!)) {
        const args = children(fields.find((field) => field.type === "arguments")!);
        const content: string[] = [],
          props: string[] = [];
        for (const argument of args) {
          const value = unwrap(argument);
          const parts = children(value);
          if (value.type === "labelled_argument") {
            if (match[1] === "swift_ui_quickgui_host_view" && parts[0]!.text === "match_contents") {
              props.push(
                `.swift_ui_match_contents_horizontal(${render(parts[1]!)}).swift_ui_match_contents_vertical(${render(parts[1]!)})`,
              );
            } else props.push(`.${parts[0]!.text}(${render(parts[1]!)})`);
          } else if (value.type === "forwarded_labelled_argument") {
            const name = parts[0]!.text.replace(/[?~]$/, "");
            props.push(`.${name}(${name})`);
          } else if (["optional_argument", "forwarded_optional_argument"].includes(value.type)) {
            const name = parts[0]!.text.replace(/[?~]$/, "");
            const expression = value.type === "optional_argument" ? render(parts[1]!) : name;
            const method = (api.methods as Record<string, { code: number }>)[name];
            if (name === "style") props.push(`.bind_optional_style(() => ${expression})`);
            else if (method) props.push(`.bind_optional(${method.code}, () => ${expression})`);
            else throw new Error(`Unsupported optional property ${filename}: ${argument.text}`);
          } else content.push(render(argument));
        }
        if (props.length) return `${fields[0]!.text}(${content.join(", ")})${props.join("")}`;
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
