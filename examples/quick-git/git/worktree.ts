/**
 * `git worktree list --porcelain -z` parsing.
 */

export interface Worktree {
  path: string;
  headSha?: string;
  /** Full ref, `refs/heads/feature`, when checked out on a branch. */
  branch?: string;
  /** Short branch name. */
  branchName?: string;
  detached: boolean;
  bare: boolean;
  locked: boolean;
  lockReason?: string;
  prunable: boolean;
  pruneReason?: string;
  /** Whether this is the main working tree (the first entry git lists). */
  main: boolean;
}

export function parseWorktrees(output: string): Worktree[] {
  const worktrees: Worktree[] = [];
  // With -z each attribute ends in NUL and entries end in an extra NUL.
  const entries = output.split("\0\0");
  for (const entry of entries) {
    const attributes = entry.split("\0").filter((line) => line.length > 0);
    if (attributes.length === 0) continue;
    const worktree: Worktree = {
      path: "",
      detached: false,
      bare: false,
      locked: false,
      prunable: false,
      main: worktrees.length === 0,
    };
    for (const attribute of attributes) {
      const space = attribute.indexOf(" ");
      const key = space < 0 ? attribute : attribute.slice(0, space);
      const value = space < 0 ? "" : attribute.slice(space + 1);
      switch (key) {
        case "worktree":
          worktree.path = value;
          break;
        case "HEAD":
          worktree.headSha = value;
          break;
        case "branch":
          worktree.branch = value;
          worktree.branchName = value.replace(/^refs\/heads\//, "");
          break;
        case "detached":
          worktree.detached = true;
          break;
        case "bare":
          worktree.bare = true;
          break;
        case "locked":
          worktree.locked = true;
          if (value) worktree.lockReason = value;
          break;
        case "prunable":
          worktree.prunable = true;
          if (value) worktree.pruneReason = value;
          break;
        default:
          break;
      }
    }
    if (worktree.path) worktrees.push(worktree);
  }
  return worktrees;
}

/** Turn a branch name into a directory name: `feature/login` becomes `feature-login`. */
export function worktreeDirectoryName(branch: string): string {
  return branch.replace(/[\/\\:\s]+/g, "-").replace(/^-+|-+$/g, "") || "worktree";
}
