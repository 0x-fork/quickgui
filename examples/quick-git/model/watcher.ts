import { FileWatcher } from "@quickgui/native";
import { relative, sep } from "node:path";

export type ChangeKind = "worktree" | "refs" | "index";

export interface RepositoryWatcherOptions {
  /** Working-tree root. */
  root: string;
  /** This worktree's own git directory (`.git` or `.git/worktrees/<name>`). */
  gitDir: string;
  /** Shared git directory, watched separately when it lies outside `root`. */
  commonDir: string;
  /** Called once per quiet period with every kind of change seen in it. */
  onChange: (kinds: ReadonlySet<ChangeKind>) => void;
  debounceMs?: number;
}

/**
 * Watch a working tree and its git directory, coalescing bursts into one callback.
 *
 * git writes lock files, temporary objects, and the index in quick succession; a 150 ms quiet
 * period folds a whole `git commit` into one refresh. Object writes are ignored outright: they
 * never change what the app shows without a ref or index change following them.
 */
export function watchRepository(options: RepositoryWatcherOptions): () => void {
  const debounceMs = options.debounceMs ?? 150;
  const pending = new Set<ChangeKind>();
  let timer: ReturnType<typeof setTimeout> | undefined;
  let closed = false;
  const flush = () => {
    timer = undefined;
    if (closed || pending.size === 0) return;
    const kinds = new Set([...pending]);
    pending.clear();
    options.onChange(kinds);
  };
  const note = (kind: ChangeKind) => {
    if (closed) return;
    pending.add(kind);
    if (timer) clearTimeout(timer);
    timer = setTimeout(flush, debounceMs);
  };
  let watcher: FileWatcher | undefined;
  const roots = [options.root];
  if (!isInside(options.gitDir, options.root)) roots.push(options.gitDir);
  if (options.commonDir !== options.gitDir && !isInside(options.commonDir, options.root)) roots.push(options.commonDir);
  const start = async (): Promise<void> => {
    try {
      const active = await FileWatcher.start(roots, (event) => {
        if (event.rescan) { note("worktree"); note("refs"); note("index"); }
        for (const path of event.paths) {
          const kind = isInside(path, options.gitDir)
            ? classifyGitPath(relative(options.gitDir, path))
            : isInside(path, options.commonDir)
              ? classifyGitPath(relative(options.commonDir, path))
              : classifyWorktreePath(relative(options.root, path), relative(options.root, options.gitDir));
          if (kind !== undefined) note(kind);
        }
      });
      if (closed) await active.close();
      else watcher = active;
    } catch (error) {
      // Focus refresh and the refresh after each write remain available if watching fails.
      console.error("Unable to watch repository", String(error));
    }
  };
  void start();

  return () => {
    closed = true;
    if (timer) clearTimeout(timer);
    const active = watcher;
    if (active !== undefined) void active.close();
  };
}

function isInside(path: string, directory: string): boolean {
  const rel = relative(directory, path);
  return rel === "" || (!rel.startsWith("..") && !rel.startsWith(sep));
}

/** Classify a path relative to the working-tree root, where `.git` may be a directory or a file. */
export function classifyWorktreePath(path: string, gitDirRelative: string): ChangeKind | undefined {
  const normalized = path.split(sep).join("/");
  const gitRelative = gitDirRelative.split(sep).join("/");
  if (normalized === ".git" || normalized === gitRelative) return "refs";
  if (normalized.startsWith(".git/")) return classifyGitPath(normalized.slice(5));
  if (gitRelative && normalized.startsWith(`${gitRelative}/`)) return classifyGitPath(normalized.slice(gitRelative.length + 1));
  return "worktree";
}

/** Classify a path relative to a git directory. */
export function classifyGitPath(path: string): ChangeKind | undefined {
  const normalized = path.split(sep).join("/");
  if (normalized.startsWith("objects/") || normalized.startsWith("lfs/") || normalized === "objects") return undefined;
  if (normalized === "index" || normalized === "index.lock") return "index";
  if (
    normalized === "HEAD" ||
    normalized === "ORIG_HEAD" ||
    normalized === "FETCH_HEAD" ||
    normalized === "MERGE_HEAD" ||
    normalized === "packed-refs" ||
    normalized.startsWith("refs/") ||
    normalized.startsWith("logs/") ||
    normalized.startsWith("worktrees/")
  ) {
    return "refs";
  }
  if (normalized.endsWith(".lock") || normalized === "COMMIT_EDITMSG") return undefined;
  return "refs";
}
