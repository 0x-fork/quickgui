import { describe, expect, test } from "bun:test";
import { mkdtemp, realpath, rm, symlink } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { canonicalPath } from "./paths.ts";

describe("canonical paths", () => {
  test("resolves symlinks so two spellings of one folder match", async () => {
    const dir = await mkdtemp(join(tmpdir(), "quick-git-paths-"));
    try {
      const link = join(dir, "link");
      await symlink(dir, link);
      expect(canonicalPath(link)).toBe(await realpath(dir));
      expect(canonicalPath(dir)).toBe(await realpath(dir));
    } finally {
      await rm(dir, { recursive: true, force: true });
    }
  });

  test("keeps a missing path absolute", () => {
    expect(canonicalPath("/definitely/missing/quick-git")).toBe("/definitely/missing/quick-git");
    expect(canonicalPath("relative/repo")).toBe(join(process.cwd(), "relative/repo"));
  });
});
