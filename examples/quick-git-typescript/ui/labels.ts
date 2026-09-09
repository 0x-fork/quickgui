import { basename, dirname } from "node:path";

/**
 * Menu labels for repository paths: the folder name, with the parent folder added only where two
 * repositories share a name, so `work/app` and `personal/app` stay tellable apart.
 */
export function repositoryLabels(paths: readonly string[]): Map<string, string> {
  const counts = new Map<string, number>();
  for (const path of paths) counts.set(basename(path), (counts.get(basename(path)) ?? 0) + 1);
  const labels = new Map<string, string>();
  for (const path of paths) {
    const name = basename(path);
    labels.set(path, (counts.get(name) ?? 0) > 1 ? `${basename(dirname(path))}/${name}` : name);
  }
  return labels;
}
