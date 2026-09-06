#!/usr/bin/env node

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const argumentsSet = new Set(process.argv.slice(2));
const supportedArguments = new Set(["--check"]);

for (const argument of argumentsSet) {
  if (!supportedArguments.has(argument)) {
    console.error(`usage: sync-release-version.mjs [--check]`);
    process.exit(2);
  }
}

const checkOnly = argumentsSet.has("--check");
const rootManifest = JSON.parse(readFileSync(join(repositoryRoot, "package.json"), "utf8"));
const version = rootManifest.version;

if (typeof version !== "string" || !/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(version)) {
  throw new Error(`package.json has an invalid release version: ${JSON.stringify(version)}`);
}

const originalFiles = new Map();
const updatedFiles = new Map();

function fileContents(relativePath) {
  if (!originalFiles.has(relativePath)) {
    const contents = readFileSync(join(repositoryRoot, relativePath), "utf8");
    originalFiles.set(relativePath, contents);
    updatedFiles.set(relativePath, contents);
  }
  return updatedFiles.get(relativePath);
}

function edit(relativePath, transform) {
  updatedFiles.set(relativePath, transform(fileContents(relativePath)));
}

function replaceMatches(relativePath, contents, pattern, replacement, expectedMatches = 1) {
  let matches = 0;
  const updated = contents.replace(pattern, (...groups) => {
    matches += 1;
    return replacement(...groups);
  });

  if (matches !== expectedMatches) {
    throw new Error(
      `${relativePath}: expected ${expectedMatches} version location${expectedMatches === 1 ? "" : "s"}, found ${matches}`,
    );
  }
  return updated;
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function replaceCargoPackageVersion(relativePath, packageName) {
  edit(relativePath, (contents) => {
    const packageStart = contents.indexOf("[package]");
    if (packageStart === -1) {
      throw new Error(`${relativePath}: missing [package] section`);
    }
    const nextSection = contents.indexOf("\n[", packageStart + "[package]".length);
    const packageEnd = nextSection === -1 ? contents.length : nextSection;
    const packageSection = contents.slice(packageStart, packageEnd);
    const namePattern = new RegExp(`^name = "${escapeRegExp(packageName)}"$`, "m");
    if (!namePattern.test(packageSection)) {
      throw new Error(`${relativePath}: [package] is not ${packageName}`);
    }

    const updatedSection = replaceMatches(
      relativePath,
      packageSection,
      /^version = "[^"]+"$/m,
      () => `version = "${version}"`,
    );
    return contents.slice(0, packageStart) + updatedSection + contents.slice(packageEnd);
  });
}

function replaceCargoSectionVersion(relativePath, sectionName) {
  edit(relativePath, (contents) => {
    const header = `[${sectionName}]`;
    const sectionStart = contents.indexOf(header);
    if (sectionStart === -1) {
      throw new Error(`${relativePath}: missing ${header} section`);
    }
    const nextSection = contents.indexOf("\n[", sectionStart + header.length);
    const sectionEnd = nextSection === -1 ? contents.length : nextSection;
    const section = contents.slice(sectionStart, sectionEnd);
    const updatedSection = replaceMatches(
      relativePath,
      section,
      /^version = "[^"]+"$/m,
      () => `version = "=${version}"`,
    );
    return contents.slice(0, sectionStart) + updatedSection + contents.slice(sectionEnd);
  });
}

function replaceInlineCargoDependency(relativePath, dependencyName) {
  edit(relativePath, (contents) => {
    const dependencyPattern = new RegExp(
      `(^${escapeRegExp(dependencyName)} = \\{[^\\n]*?version = ")([^"]+)("[^\\n]*\\}$)`,
      "m",
    );
    return replaceMatches(
      relativePath,
      contents,
      dependencyPattern,
      (_match, prefix, _oldVersion, suffix) => `${prefix}=${version}${suffix}`,
    );
  });
}

function replaceJsonPackageVersion(relativePath, packageName) {
  edit(relativePath, (contents) => {
    const manifest = JSON.parse(contents);
    if (manifest.name !== packageName) {
      throw new Error(`${relativePath}: expected package name ${packageName}, found ${manifest.name}`);
    }
    return replaceMatches(
      relativePath,
      contents,
      /^(  "version": ")[^"]+("[,]?)$/m,
      (_match, prefix, suffix) => `${prefix}${version}${suffix}`,
    );
  });
}

function replaceCargoLockPackageVersion(relativePath, packageName) {
  edit(relativePath, (contents) => {
    const pattern = new RegExp(
      `(\\[\\[package\\]\\]\\r?\\nname = "${escapeRegExp(packageName)}"\\r?\\nversion = ")[^"]+(")`,
      "g",
    );
    return replaceMatches(
      relativePath,
      contents,
      pattern,
      (_match, prefix, suffix) => `${prefix}${version}${suffix}`,
    );
  });
}

function replaceBunWorkspaceVersion(relativePath, workspacePath, packageName) {
  edit(relativePath, (contents) => {
    const pattern = new RegExp(
      `(    "${escapeRegExp(workspacePath)}": \\{\\r?\\n      "name": "${escapeRegExp(packageName)}",\\r?\\n      "version": ")[^"]+(")`,
    );
    return replaceMatches(
      relativePath,
      contents,
      pattern,
      (_match, prefix, suffix) => `${prefix}${version}${suffix}`,
    );
  });
}

const cargoPackages = [
  ["Cargo.toml", "quickgui"],
  ["crates/quickgui-system/Cargo.toml", "quickgui-system"],
  ["crates/quickgui-host/Cargo.toml", "quickgui-host"],
  ["vendor/winit/Cargo.toml", "quickgui-winit"],
  ["vendor/winit/Cargo.toml.orig", "quickgui-winit"],
  ["vendor/accesskit_winit/Cargo.toml", "quickgui-accesskit-winit"],
  ["vendor/accesskit_winit/Cargo.toml.orig", "quickgui-accesskit-winit"],
  ["vendor/cosmic_text/Cargo.toml", "quickgui-cosmic-text"],
  ["vendor/glyphon/Cargo.toml", "quickgui-glyphon"],
];

for (const [relativePath, packageName] of cargoPackages) {
  replaceCargoPackageVersion(relativePath, packageName);
}

for (const dependencyName of ["accesskit_winit", "glyphon", "quickgui-system", "winit"]) {
  replaceInlineCargoDependency("Cargo.toml", dependencyName);
}
replaceInlineCargoDependency("vendor/accesskit_winit/Cargo.toml.orig", "winit");
replaceCargoSectionVersion("vendor/accesskit_winit/Cargo.toml", "dependencies.winit");
replaceCargoSectionVersion("vendor/accesskit_winit/Cargo.toml", "dev-dependencies.winit");
replaceCargoSectionVersion("vendor/accesskit_winit/Cargo.toml.orig", "dev-dependencies.winit");
replaceInlineCargoDependency("vendor/glyphon/Cargo.toml", "cosmic-text");
replaceInlineCargoDependency("tests/downstream_smoke/Cargo.toml", "quickgui");

for (const [relativePath, packageName] of [
  ["packages/native/package.json", "@quickgui/native"],
  ["packages/ui/package.json", "@quickgui/ui"],
  ["packages/cli/package.json", "@quickgui/cli"],
]) {
  replaceJsonPackageVersion(relativePath, packageName);
}

edit("packages/cli/src/cli.ts", (contents) =>
  replaceMatches(
    "packages/cli/src/cli.ts",
    contents,
    /^export const CLI_VERSION = "[^"]+";$/m,
    () => `export const CLI_VERSION = "${version}";`,
  ),
);

for (const packageName of ["native", "ui", "cli"]) {
  edit("packages/cli/templates/native/package.json", (contents) =>
    replaceMatches(
      "packages/cli/templates/native/package.json",
      contents,
      new RegExp(`("@quickgui/${packageName}": "\\^)[^"]+(")`),
      (_match, prefix, suffix) => `${prefix}${version}${suffix}`,
    ),
  );
}

for (const packageName of ["native", "ui"]) {
  edit("packages/cli/src/cli.test.ts", (contents) =>
    replaceMatches(
      "packages/cli/src/cli.test.ts",
      contents,
      new RegExp(`("@quickgui/${packageName}": "\\^)[^"]+(")`),
      (_match, prefix, suffix) => `${prefix}${version}${suffix}`,
    ),
  );
}


for (const packageName of [
  "quickgui",
  "quickgui-winit",
  "quickgui-accesskit-winit",
  "quickgui-cosmic-text",
  "quickgui-glyphon",
  "quickgui-system",
  "quickgui-host",
]) {
  replaceCargoLockPackageVersion("Cargo.lock", packageName);
}

for (const [workspacePath, packageName] of [
  ["packages/native", "@quickgui/native"],
  ["packages/ui", "@quickgui/ui"],
  ["packages/cli", "@quickgui/cli"],
]) {
  replaceBunWorkspaceVersion("bun.lock", workspacePath, packageName);
}

const changedFiles = [...updatedFiles.keys()].filter(
  (relativePath) => updatedFiles.get(relativePath) !== originalFiles.get(relativePath),
);

if (checkOnly && changedFiles.length > 0) {
  console.error(`release version ${version} is not synchronized:`);
  for (const relativePath of changedFiles) {
    console.error(`  ${relativePath}`);
  }
  console.error("run `bun run version:sync` to update them");
  process.exit(1);
}

if (!checkOnly) {
  for (const relativePath of changedFiles) {
    writeFileSync(join(repositoryRoot, relativePath), updatedFiles.get(relativePath));
  }
}

const action = checkOnly ? "verified" : changedFiles.length > 0 ? "updated" : "already synchronized";
console.log(`release version ${version}: ${action} ${updatedFiles.size} files`);
