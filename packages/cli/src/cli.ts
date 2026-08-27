#!/usr/bin/env bun

import { resolve } from "node:path";

import { parseCliArgs, type ParsedCliCommand } from "./args.ts";
import { buildProject } from "./build.ts";
import { loadConfig } from "./config.ts";
import { runDev } from "./dev.ts";
import { CliError, errorMessage } from "./error.ts";
import { initProject } from "./init.ts";
import { hostTarget } from "./targets.ts";

export const CLI_VERSION = "0.0.1";

export async function runCli(argv: string[]): Promise<number> {
  const command = parseCliArgs(argv);
  switch (command.command) {
    case "help":
      console.log(helpText(command.topic));
      return 0;
    case "version":
      console.log(CLI_VERSION);
      return 0;
    case "init": {
      const destination = await initProject(command);
      console.log(`\nCreated QuickGUI project at ${destination}`);
      console.log(`\n  cd ${relativeDisplayPath(destination)}`);
      if (!command.install) console.log("  bun install");
      console.log("  bun run dev");
      return 0;
    }
    case "dev":
      return await runDev(command);
    case "build":
      return await runBuild(command);
  }
}

async function runBuild(
  command: Extract<ParsedCliCommand, { command: "build" }>,
): Promise<number> {
  const projectRoot = resolve(command.project);
  const config = await loadConfig(projectRoot, command.configFile);
  const target = command.target ?? config.target ?? hostTarget();
  console.log(`[quickgui] Building ${config.name} for ${target}`);
  const result = await buildProject(config, {
    mode: "production",
    target,
    ...(command.outDir ? { outDir: command.outDir } : {}),
    ...(command.signingIdentity ? { signingIdentity: command.signingIdentity } : {}),
  });
  console.log(`[quickgui] Created ${result.artifactPath}`);
  return 0;
}

function helpText(topic?: "init" | "dev" | "build"): string {
  if (topic === "init") {
    return `Usage: quickgui init [directory] [options]

Create a Solid-powered QuickGUI project.

Options:
  --name <name>              Application display name
  --identifier <id>          Reverse-DNS bundle identifier
  --no-install               Do not run bun install
  -h, --help                 Show this help`;
  }
  if (topic === "dev") {
    return `Usage: quickgui dev [options]

Package a native development host, run it, and restart it on source changes.
On macOS the host is a signed .app; application TS/TSX stays outside the bundle.

Options:
  --project <directory>      Project directory (default: .)
  --config <file>            Config file (default: quickgui.config.ts)
  --target <target>          Host target override
  --sign <identity>          macOS signing identity (default: ad-hoc)
  --once                     Run without watching
  --no-launch                Only create the development app
  -h, --help                 Show this help`;
  }
  if (topic === "build") {
    return `Usage: quickgui build [options]

Build a self-contained production application for a target platform.

Options:
  --project <directory>      Project directory (default: .)
  --config <file>            Config file (default: quickgui.config.ts)
  --target <target>          darwin-arm64, darwin-x64, linux-arm64,
                            linux-x64, windows-arm64, or windows-x64
  --out-dir <directory>      Output directory override
  --sign <identity>          macOS signing identity (default: ad-hoc)
  -h, --help                 Show this help`;
  }
  return `QuickGUI CLI ${CLI_VERSION}

Usage: quickgui <command> [options]

Commands:
  init [directory]           Create a Solid-powered project
  dev                        Run a native app with source reload
  build                      Package a production application

Run \`quickgui help <command>\` for command-specific help.`;
}

function relativeDisplayPath(path: string): string {
  const current = process.cwd();
  return path.startsWith(`${current}/`) ? path.slice(current.length + 1) : path;
}

if (import.meta.main) {
  try {
    process.exitCode = await runCli(process.argv.slice(2));
  } catch (error) {
    console.error(`quickgui: ${errorMessage(error)}`);
    process.exitCode = error instanceof CliError ? error.exitCode : 1;
  }
}
