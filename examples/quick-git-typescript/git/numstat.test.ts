import { expect, test } from "bun:test";
import { parseNumstat } from "./repository.ts";

test("numstat preserves tabbed, multiline, renamed, and binary paths", () => {
  expect(
    parseNumstat(
      "12\t3\tsrc/app.ts\0-\t-\tlogo.png\0" +
        "1\t0\twith\ttab\nand newline.txt\0" +
        "2\t1\t\0old\tname.txt\0new\tname.txt\0",
    ),
  ).toEqual([
    { path: "src/app.ts", added: 12, removed: 3 },
    { path: "logo.png", added: null, removed: null },
    { path: "with\ttab\nand newline.txt", added: 1, removed: 0 },
    { path: "new\tname.txt", added: 2, removed: 1 },
  ]);
});
