import { describe, expect, test } from "bun:test";

import {
  countChanges,
  formatPatch,
  parseDiff,
  parseGitHeaderPaths,
  selectsWholeHunk,
  syntheticAddedFile,
} from "./diff.ts";

const SAMPLE = `diff --git a/src/app.ts b/src/app.ts
index 422c2b7..6372083 100644
--- a/src/app.ts
+++ b/src/app.ts
@@ -1,4 +1,5 @@ function main() {
 line one
-line two
+line 2
+line 2b
 line three
 line four
@@ -10,3 +11,3 @@
 ten
-eleven
+11
 twelve
\\ No newline at end of file
diff --git a/README.md b/README.md
deleted file mode 100644
index 1234567..0000000
--- a/README.md
+++ /dev/null
@@ -1,2 +0,0 @@
-hello
-world
diff --git a/old dir/name.txt b/new dir/name.txt
similarity index 90%
rename from old dir/name.txt
rename to new dir/name.txt
index abc..def 100644
--- a/old dir/name.txt
+++ b/new dir/name.txt
@@ -1 +1 @@
-a
+b
diff --git a/image.png b/image.png
new file mode 100644
index 0000000..89abcde
Binary files /dev/null and b/image.png differ
diff --git a/script.sh b/script.sh
old mode 100644
new mode 100755
`;

describe("unified diff parsing", () => {
  test("parses hunks, line numbers, no-newline markers, and every file kind", () => {
    const files = parseDiff(SAMPLE);
    expect(files.map((file) => [file.kind, file.path, file.binary, file.hunks.length])).toEqual([
      ["modified", "src/app.ts", false, 2],
      ["deleted", "README.md", false, 1],
      ["renamed", "new dir/name.txt", false, 1],
      ["added", "image.png", true, 0],
      ["modified", "script.sh", false, 0],
    ]);
    const app = files[0]!;
    const first = app.hunks[0]!;
    expect(first).toMatchObject({
      oldStart: 1,
      oldLines: 4,
      newStart: 1,
      newLines: 5,
      heading: "function main() {",
    });
    expect(
      first.lines.map((line) => [line.kind, line.text, line.oldLineNumber, line.newLineNumber]),
    ).toEqual([
      ["context", "line one", 1, 1],
      ["removed", "line two", 2, null],
      ["added", "line 2", null, 2],
      ["added", "line 2b", null, 3],
      ["context", "line three", 3, 4],
      ["context", "line four", 4, 5],
    ]);
    const second = app.hunks[1]!;
    expect(second.lines.at(-1)).toMatchObject({ kind: "context", text: "twelve", noNewline: true });
    expect(countChanges(app)).toEqual({ added: 3, removed: 2 });
    expect(files[1]!.newPath).toBeNull();
    expect(files[2]).toMatchObject({
      oldPath: "old dir/name.txt",
      newPath: "new dir/name.txt",
      similarity: 90,
    });
    expect(files[3]!.oldPath).toBeNull();
    expect(files[4]).toMatchObject({ oldMode: "100644", newMode: "100755" });
  });

  test("splits diff --git headers whose paths contain spaces", () => {
    expect(parseGitHeaderPaths("a/with space.txt b/with space.txt")).toEqual({
      oldPath: "with space.txt",
      newPath: "with space.txt",
    });
    expect(parseGitHeaderPaths("a/a b/c.txt b/a b/c.txt")).toEqual({
      oldPath: "a b/c.txt",
      newPath: "a b/c.txt",
    });
    expect(parseGitHeaderPaths("a/one.txt b/two.txt")).toEqual({
      oldPath: "one.txt",
      newPath: "two.txt",
    });
  });

  test("marks a truncated diff on its last file", () => {
    const files = parseDiff(SAMPLE, { truncated: true });
    expect(files.at(-1)!.truncated).toBe(true);
    expect(files[0]!.truncated).toBe(false);
  });

  test("synthesizes an untracked file as one added hunk", () => {
    const file = syntheticAddedFile("notes.md", "a\nb");
    expect(file.kind).toBe("added");
    expect(file.hunks[0]!.lines.map((line) => line.text)).toEqual(["a", "b"]);
    expect(file.hunks[0]!.lines[1]!.noNewline).toBe(true);
    expect(syntheticAddedFile("empty.txt", "").hunks).toHaveLength(0);
  });
});

describe("patch formatting", () => {
  const app = () => parseDiff(SAMPLE)[0]!;

  test("stages a whole hunk with its own header and recounted ranges", () => {
    const patch = formatPatch(app(), [{ hunkIndex: 1 }]);
    expect(patch).toBe(
      [
        "diff --git a/src/app.ts b/src/app.ts",
        "--- a/src/app.ts",
        "+++ b/src/app.ts",
        "@@ -10,3 +10,3 @@",
        " ten",
        "-eleven",
        "+11",
        " twelve",
        "\\ No newline at end of file",
        "",
      ].join("\n"),
    );
  });

  test("stages chosen lines: unselected additions vanish, unselected removals stay as context", () => {
    // Keep only the `+line 2b` addition of the first hunk.
    const patch = formatPatch(app(), [{ hunkIndex: 0, lines: new Set([3]) }]);
    expect(patch.split("\n").slice(3)).toEqual([
      "@@ -1,4 +1,5 @@ function main() {",
      " line one",
      " line two",
      "+line 2b",
      " line three",
      " line four",
      "",
    ]);
  });

  test("unstages chosen lines in reverse: unselected additions stay, unselected removals vanish", () => {
    const patch = formatPatch(app(), [{ hunkIndex: 0, lines: new Set([1]) }], { reverse: true });
    expect(patch.split("\n").slice(3)).toEqual([
      "@@ -1,6 +1,5 @@ function main() {",
      " line one",
      "-line two",
      " line 2",
      " line 2b",
      " line three",
      " line four",
      "",
    ]);
  });

  test("carries the new-side offset of earlier included hunks into later ones", () => {
    const patch = formatPatch(app(), [{ hunkIndex: 0 }, { hunkIndex: 1 }]);
    expect(patch).toContain("@@ -1,4 +1,5 @@");
    expect(patch).toContain("@@ -10,3 +11,3 @@");
  });

  test("returns nothing when the selection changes no line", () => {
    expect(formatPatch(app(), [{ hunkIndex: 0, lines: new Set([0]) }])).toBe("");
    expect(formatPatch(app(), [{ hunkIndex: 9 }])).toBe("");
  });

  test("keeps new-file and deleted-file headers only for whole-file patches", () => {
    const files = parseDiff(SAMPLE);
    const deleted = files[1]!;
    expect(formatPatch(deleted, [{ hunkIndex: 0 }])).toContain("deleted file mode 100644");
    expect(formatPatch(deleted, [{ hunkIndex: 0 }])).toContain("+++ /dev/null");
    const partial = formatPatch(deleted, [{ hunkIndex: 0, lines: new Set([0]) }]);
    expect(partial).not.toContain("deleted file mode");
    expect(partial).toContain("--- a/README.md\n+++ b/README.md\n@@ -1,2 +1 @@\n-hello\n world\n");

    const added = syntheticAddedFile("notes.md", "a\nb\n");
    expect(formatPatch(added, [{ hunkIndex: 0, lines: new Set([0]) }])).toBe(
      "diff --git a/notes.md b/notes.md\nnew file mode 100644\n--- /dev/null\n+++ b/notes.md\n@@ -0,0 +1 @@\n+a\n",
    );
    // Unstaging part of an added file must keep the file in the index.
    const reverse = formatPatch(added, [{ hunkIndex: 0, lines: new Set([0]) }], { reverse: true });
    expect(reverse).not.toContain("new file mode");
    expect(reverse).toContain("@@ -1 +1,2 @@\n+a\n b\n");
    expect(formatPatch(added, [{ hunkIndex: 0 }], { reverse: true })).toContain(
      "new file mode 100644",
    );
  });

  test("renames keep their rename header", () => {
    const renamed = parseDiff(SAMPLE)[2]!;
    const patch = formatPatch(renamed, [{ hunkIndex: 0 }]);
    expect(
      patch.startsWith(
        "diff --git a/old dir/name.txt b/new dir/name.txt\nsimilarity index 90%\nrename from old dir/name.txt\nrename to new dir/name.txt\n",
      ),
    ).toBe(true);
  });

  test("knows when a line set covers a whole hunk", () => {
    const hunk = app().hunks[0]!;
    expect(selectsWholeHunk(hunk, new Set([1, 2, 3]))).toBe(true);
    expect(selectsWholeHunk(hunk, new Set([1, 2]))).toBe(false);
  });
});
