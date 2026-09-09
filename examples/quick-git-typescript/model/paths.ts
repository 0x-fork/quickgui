import { realpathSync } from "node:fs";
import { resolve } from "node:path";

/**
 * One name for one folder: absolute, with symlinks resolved, so `/tmp/repo` and
 * `/private/tmp/repo` count as the same repository in the recent list and the open windows.
 * A path that does not exist yet is only made absolute.
 */
export function canonicalPath(path: string): string {
  const absolute = resolve(path);
  try {
    return realpathSync(absolute);
  } catch {
    return absolute;
  }
}
