import { expect, test } from "bun:test";
import { snippets } from "../src/lib/snippets";
import { getHighlightedSnippets } from "../src/server/highlight.server";

test("the homepage can highlight every frontend, including Solid TSX", async () => {
  const highlighted = await getHighlightedSnippets();
  expect(Object.keys(highlighted)).toEqual(Object.keys(snippets));
  for (const html of Object.values(highlighted)) {
    expect(html).toContain("<pre");
    expect(html).toContain("</pre>");
  }
  expect(highlighted.typescriptCounter).toContain("createSignal");
  expect(highlighted.typescriptCounter).toContain("background-color");
  expect(snippets.typescriptCounter.code).not.toContain("backgroundColor");
  expect(snippets.typescriptCounter.code).toMatch(/\brounded-lg(?:\s|>)/);
  expect(snippets.typescriptCounter.code).not.toContain('rounded="');
});
