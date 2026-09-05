/**
 * One opened repository (or one of its worktrees) and every git operation the app performs on
 * it. Reads return parsed snapshots; writes return nothing and let the caller refresh. Every
 * method accepts an `AbortSignal` so a superseded request kills its process instead of
 * finishing work nobody will look at.
 */

import { readFile, rm, stat } from "node:fs/promises";
import { isAbsolute, join, resolve } from "node:path";

import { formatPatch, looksBinary, parseDiff, syntheticAddedFile, type DiffFile, type HunkSelection } from "./diff.ts";
import { LOG_FORMAT, parseLog, type Commit } from "./log.ts";
import {
  DEFAULT_MAX_OUTPUT_BYTES,
  GitError,
  GitRunner,
  NETWORK_GIT_TIMEOUT_MS,
  decodeOutput,
  splitNul,
  type GitPriority,
} from "./process.ts";
import { FOR_EACH_REF_FORMAT, STASH_FORMAT, parseRefs, parseStashes, type RefCollections, type StashEntry } from "./refs.ts";
import { parseStatus, type RepositoryStatus } from "./status.ts";
import { parseWorktrees, type Worktree } from "./worktree.ts";

export interface RepositoryInfo {
  /** Absolute path of the working tree root. */
  root: string;
  /** Absolute `.git` directory of this working tree (a file-backed gitdir for linked worktrees). */
  gitDir: string;
  /** Absolute shared `.git` directory: the same for every worktree of one repository. */
  commonDir: string;
  bare: boolean;
}

export interface CommandOptions {
  signal?: AbortSignal;
  priority?: GitPriority;
}

export interface LogOptions extends CommandOptions {
  limit?: number;
  skip?: number;
  /** A ref or revision range; defaults to HEAD. */
  ref?: string;
  /** Include every ref, for a whole-repository history. */
  all?: boolean;
}

export interface CommitOptions extends CommandOptions {
  amend?: boolean;
  signoff?: boolean;
  allowEmpty?: boolean;
  noVerify?: boolean;
}

export interface CommitFile {
  status: string;
  path: string;
  originalPath?: string;
}

export interface NumstatEntry {
  path: string;
  added: number | null;
  removed: number | null;
}

/** Largest working-tree file synthesized into an added-file diff. */
export const MAX_UNTRACKED_PREVIEW_BYTES = 4 * 1024 * 1024;
/** Largest patch output kept for one file. */
export const MAX_DIFF_OUTPUT_BYTES = 32 * 1024 * 1024;

export class Repository {
  readonly runner: GitRunner;
  readonly info: RepositoryInfo;
  /** Removes an untracked file when discarding it; defaults to deleting it outright. */
  readonly trash: (absolutePath: string) => Promise<void>;

  private constructor(
    runner: GitRunner,
    info: RepositoryInfo,
    trash?: (absolutePath: string) => Promise<void>,
  ) {
    this.runner = runner;
    this.info = info;
    this.trash = trash ?? ((path) => rm(path, { recursive: true, force: true }));
  }

  get root(): string {
    return this.info.root;
  }

  /** Open the repository containing `path`, which may be a subdirectory or a file inside it. */
  static async open(
    runner: GitRunner,
    path: string,
    options: CommandOptions & { trash?: (absolutePath: string) => Promise<void> } = {},
  ): Promise<Repository> {
    let cwd = resolve(path);
    try {
      const target = await stat(cwd);
      if (!target.isDirectory()) cwd = resolve(cwd, "..");
    } catch {
      throw new GitError(`${path} does not exist`, {
        command: [],
        cwd,
        exitCode: null,
        stderr: `${path} does not exist`,
      });
    }
    const output = await runner.text(
      ["rev-parse", "--show-toplevel", "--absolute-git-dir", "--git-common-dir", "--is-bare-repository"],
      { cwd, ...(options.signal ? { signal: options.signal } : {}) },
    );
    const [root, gitDir, commonDir, bare] = output.split("\n");
    if (!root || !gitDir) {
      throw new GitError("not a git repository", { command: [], cwd, exitCode: null, stderr: output });
    }
    const info: RepositoryInfo = {
      root,
      gitDir,
      commonDir: commonDir ? (isAbsolute(commonDir) ? commonDir : resolve(root, commonDir)) : gitDir,
      bare: bare === "true",
    };
    return new Repository(runner, info, options.trash);
  }

  /** A sibling worktree of this repository, sharing the runner. */
  worktree(path: string, options: CommandOptions = {}): Promise<Repository> {
    return Repository.open(this.runner, path, { ...options, trash: this.trash });
  }

  // ---------------------------------------------------------------------------------------------
  // Reads
  // ---------------------------------------------------------------------------------------------

  async status(options: CommandOptions = {}): Promise<RepositoryStatus> {
    const output = await this.runner.text(
      [
        "--no-optional-locks",
        "status",
        "--porcelain=v2",
        "-z",
        "--branch",
        "--show-stash",
        "--untracked-files=all",
        "--renames",
      ],
      this.#options(options),
    );
    return parseStatus(output);
  }

  /** Per-file line counts for the working tree (`staged: false`) or the index. */
  async numstat(staged: boolean, options: CommandOptions = {}): Promise<NumstatEntry[]> {
    const output = await this.runner.text(
      ["--no-optional-locks", "diff", "--numstat", "-z", "--find-renames", ...(staged ? ["--cached"] : [])],
      this.#options(options),
    );
    const entries: NumstatEntry[] = [];
    const records = splitNul(output);
    for (let index = 0; index < records.length; index += 1) {
      const record = records[index]!;
      const [added, removed, path] = record.split("\t");
      if (added === undefined || removed === undefined) continue;
      let filePath = path ?? "";
      if (filePath === "") {
        // A rename prints `added\tremoved\t\0old\0new\0`.
        index += 2;
        filePath = records[index] ?? "";
      }
      entries.push({
        path: filePath,
        added: added === "-" ? null : Number(added),
        removed: removed === "-" ? null : Number(removed),
      });
    }
    return entries;
  }

  /** The unstaged diff of one path (index versus working tree). */
  async diffWorkingTree(path: string, options: CommandOptions = {}): Promise<DiffFile[]> {
    return this.#diff(["diff"], [path], options);
  }

  /** The staged diff of one path (HEAD versus index), passing a rename's source too. */
  async diffIndex(path: string, originalPath: string | undefined, options: CommandOptions = {}): Promise<DiffFile[]> {
    return this.#diff(["diff", "--cached"], originalPath ? [originalPath, path] : [path], options);
  }

  /** An untracked file rendered as an addition, or `undefined` when it is not previewable. */
  async diffUntracked(path: string, options: CommandOptions = {}): Promise<DiffFile> {
    const absolute = join(this.root, path);
    const details = await stat(absolute);
    if (details.size > MAX_UNTRACKED_PREVIEW_BYTES) {
      const file = syntheticAddedFile(path, "", { binary: false });
      file.truncated = true;
      return file;
    }
    if (options.signal?.aborted) throw abortError(this.root);
    const bytes = new Uint8Array(await readFile(absolute));
    if (looksBinary(bytes)) return syntheticAddedFile(path, "", { binary: true });
    return syntheticAddedFile(path, decodeOutput(bytes));
  }

  /** Files touched by one commit, against its first parent. */
  async commitFiles(sha: string, options: CommandOptions = {}): Promise<CommitFile[]> {
    const output = await this.runner.text(
      ["show", "--format=", "--first-parent", "-M", "-z", "--name-status", sha, "--"],
      this.#options(options, "background"),
    );
    const files: CommitFile[] = [];
    const records = splitNul(output);
    for (let index = 0; index < records.length; index += 1) {
      const status = records[index]!;
      const code = status[0] ?? "";
      if (code === "R" || code === "C") {
        const originalPath = records[index + 1] ?? "";
        const path = records[index + 2] ?? "";
        index += 2;
        files.push({ status: code, path, originalPath });
      } else {
        const path = records[index + 1] ?? "";
        index += 1;
        files.push({ status: code, path });
      }
    }
    return files;
  }

  /** The patch of one commit, optionally narrowed to one path, against its first parent. */
  async diffCommit(sha: string, path: string | undefined, options: CommandOptions = {}): Promise<DiffFile[]> {
    return this.#diff(["show", "--format=", "--first-parent", sha], path === undefined ? [] : [path], options);
  }

  async log(options: LogOptions = {}): Promise<Commit[]> {
    const args = ["log", "-z", `--format=${LOG_FORMAT}`, "--date-order", `-n`, String(options.limit ?? 200)];
    if (options.skip) args.push(`--skip=${options.skip}`);
    if (options.all) args.push("--all");
    if (options.ref) args.push(options.ref);
    args.push("--");
    const result = await this.runner.run(args, {
      ...this.#options(options, "background"),
      // An empty repository has no HEAD to log; report no commits instead of failing.
      allowExitCodes: [128],
    });
    if (result.exitCode !== 0) return [];
    return parseLog(decodeOutput(result.stdout));
  }

  async refs(options: CommandOptions = {}): Promise<RefCollections> {
    const output = await this.runner.text(
      [
        "for-each-ref",
        `--format=${FOR_EACH_REF_FORMAT}`,
        "--sort=-committerdate",
        "refs/heads",
        "refs/remotes",
        "refs/tags",
      ],
      this.#options(options, "background"),
    );
    return parseRefs(output);
  }

  async stashes(options: CommandOptions = {}): Promise<StashEntry[]> {
    const output = await this.runner.text(
      ["stash", "list", "-z", `--format=${STASH_FORMAT}`],
      this.#options(options, "background"),
    );
    return parseStashes(output);
  }

  async worktrees(options: CommandOptions = {}): Promise<Worktree[]> {
    const output = await this.runner.text(["worktree", "list", "--porcelain", "-z"], this.#options(options, "background"));
    return parseWorktrees(output);
  }

  /** The most recent commit subjects, for a commit-message assistant to match the house style. */
  async recentSubjects(count: number, options: CommandOptions = {}): Promise<string[]> {
    const result = await this.runner.run(["log", "-z", "--format=%s", "-n", String(count), "--"], {
      ...this.#options(options, "background"),
      allowExitCodes: [128],
    });
    if (result.exitCode !== 0) return [];
    return splitNul(decodeOutput(result.stdout));
  }

  async config(key: string, options: CommandOptions = {}): Promise<string | undefined> {
    const result = await this.runner.run(["config", "--get", key], {
      ...this.#options(options, "background"),
      allowExitCodes: [1],
    });
    if (result.exitCode !== 0) return undefined;
    return decodeOutput(result.stdout).replace(/\n$/, "");
  }

  /** Whether HEAD names a commit, which an unborn branch's does not. */
  async hasHead(options: CommandOptions = {}): Promise<boolean> {
    const result = await this.runner.run(["rev-parse", "--verify", "--quiet", "HEAD"], {
      ...this.#options(options),
      allowExitCodes: [1],
    });
    return result.exitCode === 0;
  }

  /** The contents of `path` at `ref`, bounded. */
  async showFile(ref: string, path: string, options: CommandOptions = {}): Promise<Uint8Array> {
    const result = await this.runner.run(["show", `${ref}:${path}`], {
      ...this.#options(options),
      maxOutputBytes: MAX_DIFF_OUTPUT_BYTES,
    });
    return result.stdout;
  }

  // ---------------------------------------------------------------------------------------------
  // Writes: the working tree and the index
  // ---------------------------------------------------------------------------------------------

  async stage(paths: readonly string[], options: CommandOptions = {}): Promise<void> {
    if (paths.length === 0) return;
    await this.#pathspec(["add", "--all", "--"], paths, options);
  }

  async stageAll(options: CommandOptions = {}): Promise<void> {
    await this.runner.run(["add", "--all"], this.#options(options));
  }

  async unstage(paths: readonly string[], options: CommandOptions = {}): Promise<void> {
    if (paths.length === 0) return;
    await this.#pathspec(["reset", "-q", "--"], paths, options);
  }

  async unstageAll(options: CommandOptions = {}): Promise<void> {
    await this.runner.run(["reset", "-q"], this.#options(options));
  }

  /**
   * Throw away working-tree changes to tracked paths and remove untracked ones.
   *
   * Tracked paths return to their index content; untracked paths go through `trash`.
   */
  async discard(
    tracked: readonly string[],
    untracked: readonly string[],
    options: CommandOptions = {},
  ): Promise<void> {
    if (tracked.length > 0) {
      await this.#pathspec(["checkout", "-q", "--"], tracked, options);
    }
    for (const path of untracked) {
      if (options.signal?.aborted) throw abortError(this.root);
      await this.trash(join(this.root, path));
    }
  }

  /** Stage the chosen hunks or lines of an unstaged diff. */
  async stagePatch(file: DiffFile, selections: readonly HunkSelection[], options: CommandOptions = {}): Promise<void> {
    const patch = formatPatch(file, selections);
    if (!patch) return;
    await this.#apply(["--cached"], patch, options);
  }

  /** Unstage the chosen hunks or lines of a staged diff. */
  async unstagePatch(file: DiffFile, selections: readonly HunkSelection[], options: CommandOptions = {}): Promise<void> {
    const patch = formatPatch(file, selections, { reverse: true });
    if (!patch) return;
    await this.#apply(["--cached", "--reverse"], patch, options);
  }

  /** Throw away the chosen hunks or lines of an unstaged diff from the working tree. */
  async discardPatch(file: DiffFile, selections: readonly HunkSelection[], options: CommandOptions = {}): Promise<void> {
    const patch = formatPatch(file, selections, { reverse: true });
    if (!patch) return;
    await this.#apply(["--reverse"], patch, options);
  }

  async commit(message: string, options: CommitOptions = {}): Promise<void> {
    const args = ["commit", "-q", "-F", "-", "--cleanup=strip"];
    if (options.amend) args.push("--amend");
    if (options.signoff) args.push("--signoff");
    if (options.allowEmpty) args.push("--allow-empty");
    if (options.noVerify) args.push("--no-verify");
    await this.runner.run(args, { ...this.#options(options), stdin: message, timeoutMs: NETWORK_GIT_TIMEOUT_MS });
  }

  /** Take one side of a conflicted path and mark it resolved. */
  async resolveConflict(path: string, side: "ours" | "theirs", options: CommandOptions = {}): Promise<void> {
    await this.runner.run(["checkout", `--${side}`, "--", path], this.#options(options));
    await this.runner.run(["add", "--", path], this.#options(options));
  }

  // ---------------------------------------------------------------------------------------------
  // Writes: branches, remotes, stashes, worktrees
  // ---------------------------------------------------------------------------------------------

  async switchBranch(name: string, options: CommandOptions = {}): Promise<void> {
    await this.runner.run(["switch", name], this.#options(options));
  }

  async createBranch(
    name: string,
    details: { from?: string; checkout?: boolean } = {},
    options: CommandOptions = {},
  ): Promise<void> {
    const start = details.from ? [details.from] : [];
    if (details.checkout ?? true) await this.runner.run(["switch", "-c", name, ...start], this.#options(options));
    else await this.runner.run(["branch", name, ...start], this.#options(options));
  }

  async deleteBranch(name: string, force: boolean, options: CommandOptions = {}): Promise<void> {
    await this.runner.run(["branch", force ? "-D" : "-d", name], this.#options(options));
  }

  async renameBranch(from: string, to: string, options: CommandOptions = {}): Promise<void> {
    await this.runner.run(["branch", "-m", from, to], this.#options(options));
  }

  async checkoutCommit(sha: string, options: CommandOptions = {}): Promise<void> {
    await this.runner.run(["switch", "--detach", sha], this.#options(options));
  }

  async fetch(options: CommandOptions & { prune?: boolean } = {}): Promise<void> {
    await this.runner.run(["fetch", ...(options.prune ?? true ? ["--prune"] : []), "--no-write-fetch-head"], {
      ...this.#options(options),
      timeoutMs: NETWORK_GIT_TIMEOUT_MS,
    });
  }

  async pull(options: CommandOptions = {}): Promise<void> {
    await this.runner.run(["pull", "--no-edit"], { ...this.#options(options), timeoutMs: NETWORK_GIT_TIMEOUT_MS });
  }

  async push(
    details: { setUpstream?: { remote: string; branch: string }; forceWithLease?: boolean } = {},
    options: CommandOptions = {},
  ): Promise<void> {
    const args = ["push", "--porcelain"];
    if (details.forceWithLease) args.push("--force-with-lease");
    if (details.setUpstream) args.push("--set-upstream", details.setUpstream.remote, details.setUpstream.branch);
    await this.runner.run(args, { ...this.#options(options), timeoutMs: NETWORK_GIT_TIMEOUT_MS });
  }

  async remotes(options: CommandOptions = {}): Promise<string[]> {
    const output = await this.runner.text(["remote"], this.#options(options, "background"));
    return output.split("\n").filter((remote) => remote.length > 0);
  }

  async stashPush(
    details: { message?: string; includeUntracked?: boolean; keepIndex?: boolean } = {},
    options: CommandOptions = {},
  ): Promise<void> {
    const args = ["stash", "push", "-q"];
    if (details.includeUntracked) args.push("--include-untracked");
    if (details.keepIndex) args.push("--keep-index");
    if (details.message) args.push("-m", details.message);
    await this.runner.run(args, this.#options(options));
  }

  async stashApply(ref: string, options: CommandOptions = {}): Promise<void> {
    await this.runner.run(["stash", "apply", "-q", ref], this.#options(options));
  }

  async stashPop(ref: string, options: CommandOptions = {}): Promise<void> {
    await this.runner.run(["stash", "pop", "-q", ref], this.#options(options));
  }

  async stashDrop(ref: string, options: CommandOptions = {}): Promise<void> {
    await this.runner.run(["stash", "drop", "-q", ref], this.#options(options));
  }

  /**
   * Add a linked worktree at `path`.
   *
   * `newBranch` creates a branch (from `base`, or HEAD); `branch` checks an existing one out.
   * With neither, the worktree is detached at `base` or HEAD.
   */
  async addWorktree(
    details: { path: string; newBranch?: string; branch?: string; base?: string },
    options: CommandOptions = {},
  ): Promise<void> {
    const args = ["worktree", "add"];
    if (details.newBranch) args.push("-b", details.newBranch);
    args.push(details.path);
    if (details.branch) args.push(details.branch);
    else if (details.base) args.push(details.base);
    else if (details.newBranch === undefined) args.push("--detach", "HEAD");
    await this.runner.run(args, this.#options(options));
  }

  async removeWorktree(path: string, force: boolean, options: CommandOptions = {}): Promise<void> {
    await this.runner.run(["worktree", "remove", ...(force ? ["--force"] : []), path], this.#options(options));
  }

  async pruneWorktrees(options: CommandOptions = {}): Promise<void> {
    await this.runner.run(["worktree", "prune"], this.#options(options));
  }

  // ---------------------------------------------------------------------------------------------

  async #diff(command: readonly string[], paths: readonly string[], options: CommandOptions): Promise<DiffFile[]> {
    const result = await this.runner.run(
      [
        "--no-optional-locks",
        ...command,
        "--no-color",
        "--no-ext-diff",
        "--find-renames",
        "--unified=3",
        "--src-prefix=a/",
        "--dst-prefix=b/",
        "--",
        ...paths,
      ],
      { ...this.#options(options), maxOutputBytes: MAX_DIFF_OUTPUT_BYTES },
    );
    return parseDiff(decodeOutput(result.stdout), { truncated: result.truncated });
  }

  async #apply(flags: readonly string[], patch: string, options: CommandOptions): Promise<void> {
    await this.runner.run(["apply", ...flags, "--whitespace=nowarn", "-"], {
      ...this.#options(options),
      stdin: patch,
    });
  }

  async #pathspec(command: readonly string[], paths: readonly string[], options: CommandOptions): Promise<void> {
    // Any number of paths, with any bytes in them, through stdin rather than the argument list.
    const args = [...command.slice(0, -1), "--pathspec-from-file=-", "--pathspec-file-nul", ...command.slice(-1)];
    await this.runner.run(args, { ...this.#options(options), stdin: `${paths.join("\0")}\0` });
  }

  #options(options: CommandOptions, priority: GitPriority = "interactive") {
    return {
      cwd: this.root,
      priority: options.priority ?? priority,
      ...(options.signal ? { signal: options.signal } : {}),
      maxOutputBytes: DEFAULT_MAX_OUTPUT_BYTES,
      env: { GIT_EDITOR: "true" },
    };
  }
}

function abortError(cwd: string): GitError {
  return new GitError("cancelled", { command: [], cwd, exitCode: null, stderr: "", aborted: true });
}
