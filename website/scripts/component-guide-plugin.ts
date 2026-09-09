type Node = { type: string; depth?: number; value?: string; children?: Node[] };
const text = (node: Node): string => node.value ?? node.children?.map(text).join("") ?? "";

// Component routes supply their own lead, preview, guidance, and complete API.
// Keep the editorial summaries in MDX as inputs to API descriptions and translation.
export function componentGuidePlugin() {
  return (tree: Node, file: { path?: string }) => {
    const path = file.path?.replaceAll("\\", "/");
    if (!path?.includes("/content/docs/") || !path.includes("/components/"))
      return;
    const children = tree.children ?? [];
    if (children[0]?.type === "paragraph") children.shift();
    const labels = ["Key props", "主要属性", "主なプロパティ"];
    const index = children.findIndex(
      (node) =>
        node.type === "heading" && node.depth === 2 && labels.includes(text(node) as string),
    );
    if (index >= 0) children.splice(index);
  };
}
