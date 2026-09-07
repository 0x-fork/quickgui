#!/usr/bin/env bun
import { appendFileSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const version = JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version as string;
const child = Bun.spawnSync(["bun", join(import.meta.dir, "sync-release-version.ts"), "--check"], {
  cwd: root,
  stdout: "pipe",
  stderr: "inherit",
});
if (child.exitCode !== 0) process.exit(child.exitCode);
const tag =
  process.env.QUICKGUI_RELEASE_TAG ||
  (process.env.GITHUB_REF_TYPE === "tag" ? process.env.GITHUB_REF_NAME : "") ||
  "";
if (tag) {
  if (tag !== `v${version}`) throw new Error(`Tag ${tag} does not match root version ${version}`);
  const heading = new RegExp(
    `^## ${version.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")} - \\d{4}-\\d{2}-\\d{2}$`,
    "m",
  );
  if (!heading.test(readFileSync(join(root, "CHANGELOG.md"), "utf8")))
    throw new Error(`CHANGELOG.md has no dated ${version} release section`);
}
const output = process.argv[2] || process.env.GITHUB_OUTPUT;
if (output) appendFileSync(output, `version=${version}\n`);
console.log(`QUICKGUI_RELEASE_METADATA ${JSON.stringify({ version, tag, passed: true })}`);
