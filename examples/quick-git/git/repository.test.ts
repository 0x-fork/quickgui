import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { mkdtemp, rm, writeFile, mkdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { GitError, GitRunner } from "./process.ts";
import { Repository } from "./repository.ts";
import { stagedChanges, unstagedChanges } from "./status.ts";

const runner = new GitRunner({ concurrency: 2 });
let root = "";
let repo: Repository;

async function git(...args: string[]): Promise<string> {
  return runner.text(args, { cwd: root });
}

beforeAll(async () => {
  root = await mkdtemp(join(tmpdir(), "quick-git-"));
  await git("init", "-q", "-b", "main");
  await git("config", "user.email", "test@example.com");
  await git("config", "user.name", "Quick Git Tests");
  await git("config", "commit.gpgsign", "false");
  repo = await Repository.open(runner, root);
});

afterAll(async () => {
  await rm(root, { recursive: true, force: true });
});

describe("Repository", () => {
  test("opens the repository from a subdirectory and reports an unborn branch", async () => {
    await mkdir(join(root, "src"), { recursive: true });
    const nested = await Repository.open(runner, join(root, "src"));
    expect(nested.root).toBe(await realRoot());
    const status = await repo.status();
    expect(status.headSha).toBeUndefined();
    expect(status.branch).toBe("main");
    expect(await repo.hasHead()).toBe(false);
    expect(await repo.log()).toEqual([]);
  });

  test("stages, unstages, previews untracked files, and commits", async () => {
    await writeFile(join(root, "src", "app.ts"), `${Array.from({ length: 20 }, (_, index) => `line ${index + 1}`).join("\n")}\n`);
    await writeFile(join(root, "with space.md"), "# Notes\n");
    let status = await repo.status();
    expect(unstagedChanges(status).map((item) => item.path).sort()).toEqual(["src/app.ts", "with space.md"]);

    const preview = await repo.diffUntracked("with space.md");
    expect(preview.kind).toBe("added");
    expect(preview.hunks[0]!.lines.map((line) => line.text)).toEqual(["# Notes"]);

    await repo.stage(["src/app.ts", "with space.md"]);
    status = await repo.status();
    expect(stagedChanges(status).map((item) => item.path).sort()).toEqual(["src/app.ts", "with space.md"]);
    expect(unstagedChanges(status)).toEqual([]);

    // Unstaging on an unborn branch works too.
    await repo.unstage(["with space.md"]);
    status = await repo.status();
    expect(stagedChanges(status).map((item) => item.path)).toEqual(["src/app.ts"]);
    await repo.stage(["with space.md"]);

    await repo.commit("Initial commit\n\nWith a body.");
    status = await repo.status();
    expect(status.headSha).toBeDefined();
    expect(status.entries).toEqual([]);
    const log = await repo.log();
    expect(log).toHaveLength(1);
    expect(log[0]).toMatchObject({ subject: "Initial commit", body: "With a body.", parents: [] });
    expect(log[0]!.refs.some((ref) => ref.kind === "branch" && ref.name === "main" && ref.current)).toBe(true);
    expect(await repo.recentSubjects(5)).toEqual(["Initial commit"]);
    expect(await repo.config("user.name")).toBe("Quick Git Tests");
    expect(await repo.config("does.not.exist")).toBeUndefined();
  });

  test("stages and unstages single hunks and lines through generated patches", async () => {
    const lines = Array.from({ length: 20 }, (_, index) => `line ${index + 1}`);
    lines[0] = "line one!";
    lines.splice(10, 0, "x", "x", "x");
    lines.push("tail added");
    await writeFile(join(root, "src", "app.ts"), `${lines.join("\n")}\n`);
    // Three hunks: the top edit, the inserted x lines, and the tail line.
    let files = await repo.diffWorkingTree("src/app.ts");
    expect(files).toHaveLength(1);
    const file = files[0]!;
    expect(file.hunks).toHaveLength(3);
    const hunkWith = (candidate: typeof file, text: string) =>
      candidate.hunks.findIndex((hunk) => hunk.lines.some((line) => line.kind === "added" && line.text === text));

    // Stage only the tail hunk.
    await repo.stagePatch(file, [{ hunkIndex: hunkWith(file, "tail added") }]);
    let staged = await repo.diffIndex("src/app.ts", undefined);
    expect(staged[0]!.hunks).toHaveLength(1);
    expect(staged[0]!.hunks[0]!.lines.filter((line) => line.kind === "added").map((line) => line.text)).toEqual([
      "tail added",
    ]);
    let unstagedFiles = await repo.diffWorkingTree("src/app.ts");
    expect(unstagedFiles[0]!.hunks).toHaveLength(2);
    expect(hunkWith(unstagedFiles[0]!, "tail added")).toBe(-1);

    // Stage one line pair of the top hunk: the `line 1` replacement only.
    const remaining = unstagedFiles[0]!;
    const topIndex = hunkWith(remaining, "line one!");
    const hunk = remaining.hunks[topIndex]!;
    const lineOneRemoved = hunk.lines.findIndex((line) => line.kind === "removed" && line.text === "line 1");
    const lineOneAdded = hunk.lines.findIndex((line) => line.kind === "added" && line.text === "line one!");
    expect(lineOneRemoved).toBeGreaterThanOrEqual(0);
    expect(lineOneAdded).toBeGreaterThanOrEqual(0);
    await repo.stagePatch(remaining, [{ hunkIndex: topIndex, lines: new Set([lineOneRemoved, lineOneAdded]) }]);
    staged = await repo.diffIndex("src/app.ts", undefined);
    const stagedText = staged[0]!.hunks.flatMap((hunk) => hunk.lines.filter((line) => line.kind !== "context").map((line) => `${line.kind}:${line.text}`));
    expect(stagedText).toEqual(["removed:line 1", "added:line one!", "added:tail added"]);

    // Unstage the tail hunk again from the staged diff.
    const tailHunkIndex = staged[0]!.hunks.findIndex((hunk) => hunk.lines.some((line) => line.text === "tail added"));
    await repo.unstagePatch(staged[0]!, [{ hunkIndex: tailHunkIndex }]);
    staged = await repo.diffIndex("src/app.ts", undefined);
    expect(staged[0]!.hunks.flatMap((hunk) => hunk.lines.filter((line) => line.kind === "added").map((line) => line.text))).toEqual(["line one!"]);

    // Discard the x-lines hunk from the working tree; the file keeps the staged edit and the tail.
    unstagedFiles = await repo.diffWorkingTree("src/app.ts");
    const xHunk = unstagedFiles[0]!.hunks.findIndex((hunk) => hunk.lines.some((line) => line.kind === "added" && line.text === "x"));
    expect(xHunk).toBeGreaterThanOrEqual(0);
    await repo.discardPatch(unstagedFiles[0]!, [{ hunkIndex: xHunk }]);
    const content = await Bun.file(join(root, "src", "app.ts")).text();
    expect(content).not.toContain("\nx\n");
    expect(content).toContain("line one!\n");
    expect(content).toContain("tail added\n");
    await repo.unstageAll();
    files = await repo.diffWorkingTree("src/app.ts");
    expect(files[0]!.hunks.length).toBeGreaterThan(0);
  });

  test("discards tracked changes and trashes untracked files through the injected remover", async () => {
    const trashed: string[] = [];
    const custom = await Repository.open(runner, root, {
      trash: async (path) => {
        trashed.push(path);
        await rm(path, { force: true });
      },
    });
    await writeFile(join(root, "junk.txt"), "junk\n");
    await custom.discard(["src/app.ts"], ["junk.txt"]);
    expect(trashed).toEqual([join(await realRoot(), "junk.txt")]);
    const status = await repo.status();
    expect(status.entries).toEqual([]);
  });

  test("manages branches, stashes, and worktrees", async () => {
    await repo.createBranch("feature/login", { checkout: false });
    let refs = await repo.refs();
    expect(refs.local.map((branch) => branch.name).sort()).toEqual(["feature/login", "main"]);
    expect(refs.local.find((branch) => branch.name === "main")?.current).toBe(true);

    await writeFile(join(root, "stash-me.txt"), "wip\n");
    await repo.stashPush({ message: "my stash", includeUntracked: true });
    const stashes = await repo.stashes();
    expect(stashes).toHaveLength(1);
    expect(stashes[0]).toMatchObject({ ref: "stash@{0}", branch: "main", summary: "my stash" });
    await repo.stashPop("stash@{0}");
    expect((await repo.status()).entries.map((entry) => entry.path)).toEqual(["stash-me.txt"]);
    await rm(join(root, "stash-me.txt"));

    const worktreePath = join(root, "..", `${root.split("/").pop()}-feature`);
    await repo.addWorktree({ path: worktreePath, branch: "feature/login" });
    let worktrees = await repo.worktrees();
    expect(worktrees.map((worktree) => [worktree.branchName, worktree.main])).toEqual([
      ["main", true],
      ["feature/login", false],
    ]);
    refs = await repo.refs();
    expect(refs.local.find((branch) => branch.name === "feature/login")?.worktreePath).toBe(worktrees[1]!.path);

    const linked = await repo.worktree(worktreePath);
    expect(linked.info.commonDir).toBe(repo.info.commonDir);
    await writeFile(join(worktreePath, "feature.txt"), "feature\n");
    await linked.stage(["feature.txt"]);
    await linked.commit("Feature work");
    expect((await linked.log())[0]!.subject).toBe("Feature work");
    expect((await repo.log())[0]!.subject).toBe("Initial commit");

    await repo.removeWorktree(worktreePath, true);
    worktrees = await repo.worktrees();
    expect(worktrees).toHaveLength(1);
    await repo.deleteBranch("feature/login", true);
    refs = await repo.refs();
    expect(refs.local.map((branch) => branch.name)).toEqual(["main"]);
  });

  test("reports commit files and diffs against the first parent", async () => {
    const [head] = await repo.log({ limit: 1 });
    const files = await repo.commitFiles(head!.sha);
    expect(files.map((file) => [file.status, file.path]).sort()).toEqual([
      ["A", "src/app.ts"],
      ["A", "with space.md"],
    ]);
    const diff = await repo.diffCommit(head!.sha, "src/app.ts");
    expect(diff).toHaveLength(1);
    expect(diff[0]!.kind).toBe("added");
  });

  test("surfaces git failures as typed errors and honours cancellation", async () => {
    const failure = await repo.switchBranch("does-not-exist").catch((error: unknown) => error);
    expect(failure).toBeInstanceOf(GitError);
    expect((failure as GitError).exitCode).not.toBe(0);
    expect((failure as GitError).summary.length).toBeGreaterThan(0);

    const controller = new AbortController();
    controller.abort();
    const cancelled = await repo.status({ signal: controller.signal }).catch((error: unknown) => error);
    expect(cancelled).toBeInstanceOf(GitError);
    expect((cancelled as GitError).aborted).toBe(true);
  });
});

async function realRoot(): Promise<string> {
  return (await runner.text(["rev-parse", "--show-toplevel"], { cwd: root })).trim();
}
