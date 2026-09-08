import { Language, Parser, type Node, type Tree } from "web-tree-sitter";
import { fileURLToPath } from "node:url";
import { CliError } from "../error.ts";

let ready: Promise<Language> | undefined;
export async function createMoonbitParser(): Promise<Parser> {
  ready ??= (async () => {
    await Parser.init();
    return Language.load(fileURLToPath(new URL("./parser/moonbit.wasm", import.meta.url)));
  })();
  const language = await ready;
  const parser = new Parser();
  parser.setLanguage(language);
  return parser;
}

export function parseMoonbit(parser: Parser, source: string, filename: string): Tree {
  const tree = parser.parse(source);
  if (!tree) throw new CliError(`${filename}: MoonBit parser did not produce a syntax tree`);
  if (tree.rootNode.hasError) {
    const visit = (node: Node): Node | undefined => {
      if (node.isError || node.isMissing) return node;
      for (const child of node.children) {
        if (child.hasError || child.isMissing) {
          const error = visit(child);
          if (error) return error;
        }
      }
    };
    const error = visit(tree.rootNode) ?? tree.rootNode;
    const { row, column } = error.startPosition;
    const near = error.text.slice(0, 80).replaceAll(/\s+/g, " ");
    tree.delete();
    throw new CliError(
      `${filename}:${row + 1}:${column + 1}: Cannot parse MoonBit view syntax near ${JSON.stringify(near)}. Update the QuickGUI CLI if using newer MoonBit syntax.`,
    );
  }
  return tree;
}

export function children(node: Node): Node[] {
  return node.namedChildren.filter((child) => !["comment", "block_comment"].includes(child.type));
}

export function unwrap(node: Node): Node {
  const nested = children(node);
  return ["argument", "atomic_expression", "literal", "parenthesized_expression"].includes(
    node.type,
  ) && nested.length === 1
    ? unwrap(nested[0]!)
    : node;
}
