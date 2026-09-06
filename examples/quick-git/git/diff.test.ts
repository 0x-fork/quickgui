import { describe, expect, test } from "bun:test";

import {
  Diff,
  countChanges,
  formatPatch,
  parseDiff,
  parseGitHeaderPaths,
  selectsWholeHunk,
  syntheticAddedFile,
  syntheticDiffText,
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
    expect(first).toMatchObject({ oldStart: 1, oldLines: 4, newStart: 1, newLines: 5, heading: "function main() {" });
    expect(first.lines.map((line) => [line.kind, line.text, line.oldLineNumber, line.newLineNumber])).toEqual([
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
    expect(files[2]).toMatchObject({ oldPath: "old dir/name.txt", newPath: "new dir/name.txt", similarity: 90 });
    expect(files[3]!.oldPath).toBeNull();
    expect(files[4]).toMatchObject({ oldMode: "100644", newMode: "100755" });
  });

  test("splits diff --git headers whose paths contain spaces", () => {
    expect(parseGitHeaderPaths("a/with space.txt b/with space.txt")).toEqual({
      oldPath: "with space.txt",
      newPath: "with space.txt",
    });
    expect(parseGitHeaderPaths("a/a b/c.txt b/a b/c.txt")).toEqual({ oldPath: "a b/c.txt", newPath: "a b/c.txt" });
    expect(parseGitHeaderPaths("a/one.txt b/two.txt")).toEqual({ oldPath: "one.txt", newPath: "two.txt" });
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
    const patch = formatPatch(app(), [{ hunkIndex: 0, lines: [3] }]);
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
    const patch = formatPatch(app(), [{ hunkIndex: 0, lines: [1] }], { reverse: true });
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
    expect(formatPatch(app(), [{ hunkIndex: 0, lines: [0] }])).toBe("");
    expect(formatPatch(app(), [{ hunkIndex: 9 }])).toBe("");
  });

  test("keeps new-file and deleted-file headers only for whole-file patches", () => {
    const files = parseDiff(SAMPLE);
    const deleted = files[1]!;
    expect(formatPatch(deleted, [{ hunkIndex: 0 }])).toContain("deleted file mode 100644");
    expect(formatPatch(deleted, [{ hunkIndex: 0 }])).toContain("+++ /dev/null");
    const partial = formatPatch(deleted, [{ hunkIndex: 0, lines: [0] }]);
    expect(partial).not.toContain("deleted file mode");
    expect(partial).toContain("--- a/README.md\n+++ b/README.md\n@@ -1,2 +1 @@\n-hello\n world\n");

    const added = syntheticAddedFile("notes.md", "a\nb\n");
    expect(formatPatch(added, [{ hunkIndex: 0, lines: [0] }])).toBe(
      "diff --git a/notes.md b/notes.md\nnew file mode 100644\n--- /dev/null\n+++ b/notes.md\n@@ -0,0 +1 @@\n+a\n",
    );
    // Unstaging part of an added file must keep the file in the index.
    const reverse = formatPatch(added, [{ hunkIndex: 0, lines: [0] }], { reverse: true });
    expect(reverse).not.toContain("new file mode");
    expect(reverse).toContain("@@ -1 +1,2 @@\n+a\n b\n");
    expect(formatPatch(added, [{ hunkIndex: 0 }], { reverse: true })).toContain("new file mode 100644");
  });

  test("renames keep their rename header", () => {
    const renamed = parseDiff(SAMPLE)[2]!;
    const patch = formatPatch(renamed, [{ hunkIndex: 0 }]);
    expect(patch.startsWith("diff --git a/old dir/name.txt b/new dir/name.txt\nsimilarity index 90%\nrename from old dir/name.txt\nrename to new dir/name.txt\n")).toBe(true);
  });

  test("knows when a line set covers a whole hunk", () => {
    const hunk = app().hunks[0]!;
    expect(selectsWholeHunk(hunk, [1, 2, 3])).toBe(true);
    expect(selectsWholeHunk(hunk, [1, 2])).toBe(false);
  });
});

describe("retained diffs", () => {
  test("parses once and answers the table with rows and counts", () => {
    const before = Diff.openCount();
    const diff = Diff.openSync(SAMPLE);
    expect(Diff.openCount()).toBe(before + 1);
    expect(diff.files.map((file) => [file.kind, file.path, file.binary, file.hunkCount, file.added, file.removed])).toEqual([
      ["modified", "src/app.ts", false, 2, 3, 2],
      ["deleted", "README.md", false, 1, 0, 2],
      ["renamed", "new dir/name.txt", false, 1, 1, 1],
      ["added", "image.png", true, 0, 0, 0],
      ["modified", "script.sh", false, 0, 0, 0],
    ]);
    expect(diff.files[2]).toMatchObject({ oldPath: "old dir/name.txt", similarity: 90 });
    expect(diff.files[4]).toMatchObject({ oldMode: "100644", newMode: "100755" });
    expect([diff.added, diff.removed]).toEqual([4, 5]);
    // 5 file headers, 4 hunk headers, 14 lines, and the binary notice.
    expect(diff.rowCount).toBe(5 + 4 + 14 + 1);

    const rows = diff.rows(0, 4);
    expect(rows.map((row) => [row.kind, row.text])).toEqual([
      ["file", "src/app.ts"],
      ["hunk", "@@ -1,4 +1,5 @@ function main() {"],
      ["line", "line one"],
      ["line", "line two"],
    ]);
    expect(rows[3]).toMatchObject({ lineKind: "removed", fileIndex: 0, hunkIndex: 0, lineIndex: 1, oldLineNumber: 2, newLineNumber: null });
    expect(diff.rows(8, 12).map((row) => [row.kind, row.text, row.noNewline])).toEqual([
      ["hunk", "@@ -10,3 +11,3 @@", false],
      ["line", "ten", false],
      ["line", "eleven", false],
      ["line", "11", false],
    ]);
    expect(diff.rows(12, 13)[0]).toMatchObject({ kind: "line", text: "twelve", noNewline: true });
    expect(diff.rows(17, 19).map((row) => [row.kind, row.text])).toEqual([
      ["file", "new dir/name.txt"],
      ["hunk", "@@ -1,1 +1,1 @@"],
    ]);
    expect(diff.rows(21, 24).map((row) => [row.kind, row.text])).toEqual([
      ["file", "image.png"],
      ["notice", "Binary file"],
      ["file", "script.sh"],
    ]);
    expect(diff.rows(100, 200)).toEqual([]);
    expect(diff.rows(3, 1)).toEqual([]);

    // The full file only materializes for patch formatting and matches the eager parser.
    expect(diff.file(0)).toEqual(parseDiff(SAMPLE)[0]!);
    expect(diff.file(9)).toBeUndefined();

    diff.close();
    expect(diff.closed).toBe(true);
    expect(Diff.openCount()).toBe(before);
    expect(diff.rows(0, 4)).toEqual([]);
    expect(diff.file(0)).toBeUndefined();
    diff.close();
  });

  test("groups the changed lines of a selection by hunk, skipping context and headers", () => {
    const diff = Diff.openSync(SAMPLE);
    // Rows 1..7 are the first hunk: header, context, removed, added, added, context, context;
    // rows 8..12 the second: header, context, removed, added, context.
    expect(diff.selectedLines([[1, 4], [6, 7], [10, 11]])).toEqual([
      { fileIndex: 0, hunkIndex: 0, lines: new Set([1, 2]) },
      { fileIndex: 0, hunkIndex: 1, lines: new Set([1, 2]) },
    ]);
    expect(diff.selectedLines([[11, 11], [3, 3], [3, 4]])).toEqual([
      { fileIndex: 0, hunkIndex: 1, lines: new Set([2]) },
      { fileIndex: 0, hunkIndex: 0, lines: new Set([1, 2]) },
    ]);
    expect(diff.selectedLines([[1, 1], [0, 0]])).toEqual([]);
    expect(diff.selectedLines([])).toEqual([]);
    diff.close();
  });

  test("marks truncation, caps rows, and renders prompt text", () => {
    const capped = Diff.openSync(SAMPLE, { truncated: true, maxRows: 4 });
    expect(capped.files.at(-1)!.truncated).toBe(true);
    const rows = capped.rows(0, capped.rowCount);
    expect(rows.filter((row) => row.kind === "line")).toHaveLength(2);
    expect(rows.at(-1)).toMatchObject({ kind: "notice", text: "Diff truncated: too many lines to show." });
    expect(rows.filter((row) => row.kind === "notice").map((row) => row.text)).toContain(
      "Diff truncated: the file is too large to show in full.",
    );
    expect(capped.added).toBe(4);
    capped.close();

    const diff = Diff.openSync(SAMPLE);
    expect(diff.render().split("\n").slice(0, 5)).toEqual([
      "diff --git a/src/app.ts b/src/app.ts",
      "@@ -1,4 +1,5 @@ function main() {",
      " line one",
      "-line two",
      "+line 2",
    ]);
    expect(diff.render()).toContain("diff --git a/image.png b/image.png\nnew file mode 100644\n--- /dev/null\n+++ b/image.png");
    diff.close();
  });

  test("opens from the raw bytes git wrote", async () => {
    const diff = await Diff.open(new TextEncoder().encode(SAMPLE));
    expect(diff.rowCount).toBe(24);
    expect(diff.render().startsWith("diff --git a/src/app.ts b/src/app.ts")).toBe(true);
    diff.close();
    expect(diff.render()).toBe("");
  });

  test("reads a big diff in chunks without changing what it sees", async () => {
    // Nearly three megabytes: several scan chunks, with files straddling their boundaries.
    const files: string[] = [];
    for (let index = 0; index < 600; index += 1) {
      const body = Array.from({ length: 60 }, (_, line) => ` context ${index}-${line} ${"x".repeat(60)}`).join("\n");
      files.push(
        `diff --git a/file-${index}.txt b/file-${index}.txt\n--- a/file-${index}.txt\n+++ b/file-${index}.txt\n` +
          `@@ -1,61 +1,61 @@ fn ${index}\n-old ${index}\n+new ${index}\n${body}\n`,
      );
    }
    const text = files.join("");
    expect(text.length).toBeGreaterThan(2 * 1_000_000);

    const chunked = await Diff.open(text);
    const whole = Diff.openSync(text);
    expect(chunked.rowCount).toBe(whole.rowCount);
    expect([chunked.added, chunked.removed]).toEqual([600, 600]);
    expect(chunked.files.length).toBe(600);
    expect(chunked.files.at(-1)).toEqual(whole.files.at(-1)!);
    // A window in the middle, where a chunk boundary falls, and the last rows.
    expect(chunked.rows(15_000, 15_004)).toEqual(whole.rows(15_000, 15_004));
    expect(chunked.rows(chunked.rowCount - 3, chunked.rowCount)).toEqual(whole.rows(whole.rowCount - 3, whole.rowCount));
    expect(chunked.file(599)).toEqual(whole.file(599)!);
    chunked.close();
    whole.close();
  });

  test("abandons a scan whose selection moved on", async () => {
    const text = `${"diff --git a/a.txt b/a.txt\n--- a/a.txt\n+++ b/a.txt\n@@ -1,2 +1,2 @@\n one\n-two\n+2\n"}${" \n".repeat(1_000_000)}`;
    const controller = new AbortController();
    const before = Diff.openCount();
    const opening = Diff.open(text, { signal: controller.signal });
    controller.abort();
    await expect(opening).rejects.toThrow("abandoned");
    expect(Diff.openCount()).toBe(before);
  });

  test("renders an untracked file as the diff text of an addition", () => {
    const text = syntheticDiffText("notes.md", "a\nb");
    expect(text).toBe("diff --git a/notes.md b/notes.md\nnew file mode 100644\n@@ -0,0 +1,2 @@\n+a\n+b\n\\ No newline at end of file\n");
    expect(parseDiff(text)).toEqual([syntheticAddedFile("notes.md", "a\nb")]);
    expect(parseDiff(syntheticDiffText("notes.md", "a\nb\n"))).toEqual([syntheticAddedFile("notes.md", "a\nb\n")]);
    expect(parseDiff(syntheticDiffText("empty.txt", ""))).toEqual([syntheticAddedFile("empty.txt", "")]);
    expect(parseDiff(syntheticDiffText("blob.bin", "", { binary: true }))).toEqual([
      syntheticAddedFile("blob.bin", "", { binary: true }),
    ]);
    const withSpace = Diff.openSync(syntheticDiffText("with space.md", "# Notes\n"));
    expect(withSpace.files[0]).toMatchObject({ kind: "added", path: "with space.md", oldPath: null, newPath: "with space.md" });
    withSpace.close();
  });
});
