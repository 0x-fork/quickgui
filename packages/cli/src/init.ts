import {
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { resolveConfig } from "./config.ts";
import { CliError } from "./error.ts";

export interface InitProjectOptions {
  directory: string;
  install: boolean;
  name?: string;
  identifier?: string;
}

const templateRoot = fileURLToPath(new URL("../templates/solid/", import.meta.url));
const templateFiles = [
  ["package.json", "package.json"],
  ["quickgui.config.ts", "quickgui.config.ts"],
  ["tsconfig.json", "tsconfig.json"],
  ["gitignore", ".gitignore"],
  ["README.md", "README.md"],
  ["src/app.tsx", "src/app.tsx"],
] as const;

export async function initProject(options: InitProjectOptions): Promise<string> {
  const destination = resolve(options.directory);
  if (existsSync(destination) && !statSync(destination).isDirectory()) {
    throw new CliError(`Project destination is not a directory: ${destination}`);
  }
  if (existsSync(destination) && readdirSync(destination).length > 0) {
    throw new CliError(`Project destination is not empty: ${destination}`);
  }

  const defaultName = displayName(basename(destination));
  const name = options.name?.trim() || defaultName;
  const identifier = options.identifier?.trim() || `com.example.${identifierSegment(name)}`;
  resolveConfig({ name, identifier }, destination);

  const replacements: Record<string, string> = {
    "{{APP_NAME}}": JSON.stringify(name),
    "{{IDENTIFIER}}": JSON.stringify(identifier),
    "{{PACKAGE_NAME}}": JSON.stringify(packageName(name)),
    "{{README_TITLE}}": name.replaceAll("\n", " ").replaceAll("\r", " "),
  };

  mkdirSync(destination, { recursive: true });
  for (const [sourceName, targetName] of templateFiles) {
    const source = join(templateRoot, sourceName);
    if (!existsSync(source)) throw new CliError(`CLI template is missing: ${source}`);
    const target = join(destination, targetName);
    mkdirSync(dirname(target), { recursive: true });
    let contents = readFileSync(source, "utf8");
    for (const [token, value] of Object.entries(replacements)) {
      contents = contents.replaceAll(token, value);
    }
    writeFileSync(target, contents);
  }

  if (options.install) {
    const child = Bun.spawn(["bun", "install"], {
      cwd: destination,
      stdin: "inherit",
      stdout: "inherit",
      stderr: "inherit",
    });
    const status = await child.exited;
    if (status !== 0) {
      throw new CliError(
        `Project created at ${destination}, but \`bun install\` failed with status ${status}`,
      );
    }
  }

  return destination;
}

function displayName(value: string): string {
  const words = value
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .split(/[^A-Za-z0-9]+/)
    .filter(Boolean);
  return words.length > 0
    ? words.map((word) => word[0]!.toUpperCase() + word.slice(1)).join(" ")
    : "QuickGUI App";
}

function packageName(value: string): string {
  const name = value
    .normalize("NFKD")
    .toLowerCase()
    .replace(/[^a-z0-9._-]+/g, "-")
    .replace(/^[._-]+|[._-]+$/g, "")
    .slice(0, 214);
  if (!name) throw new CliError(`Could not derive a package name from ${JSON.stringify(value)}`);
  return name;
}

function identifierSegment(value: string): string {
  return packageName(value).replace(/[._]+/g, "-");
}
