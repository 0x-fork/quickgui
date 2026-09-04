#!/usr/bin/env bun

import { existsSync } from "node:fs";
import { resolve } from "node:path";

import { parseCliArgs, type ParsedCliCommand } from "./args.ts";
import { buildProject } from "./build.ts";
import { loadConfig } from "./config.ts";
import { runDev } from "./dev.ts";
import { CliError, errorMessage } from "./error.ts";
import { initProject } from "./init.ts";
import { findMinisignTool } from "./packaging/pipeline.ts";
import { minisignKeygenArguments } from "./packaging/updates.ts";
import { hostTarget } from "./targets.ts";

export const CLI_VERSION = "0.1.2";

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
    case "keygen":
      return await runKeygen(command);
  }
}

async function runKeygen(
  command: Extract<ParsedCliCommand, { command: "keygen" }>,
): Promise<number> {
  const tool = findMinisignTool();
  if (!tool) {
    throw new CliError(
      "Creating an update signing key needs `minisign` or `rsign` on PATH. Install one from " +
        "https://jedisct1.github.io/minisign/ and re-run `quickgui keygen`.",
    );
  }
  const outDir = resolve(command.outDir);
  const publicKeyPath = resolve(outDir, "quickgui-update.pub");
  const secretKeyPath = resolve(outDir, "quickgui-update.key");
  for (const path of [publicKeyPath, secretKeyPath]) {
    if (existsSync(path) && !command.force) {
      throw new CliError(`${path} already exists. Pass --force to overwrite it.`);
    }
  }
  const argv = minisignKeygenArguments(
    tool,
    publicKeyPath,
    secretKeyPath,
    command.passwordless,
  );
  const child = Bun.spawn(argv, { cwd: outDir, stdin: "inherit", stdout: "inherit", stderr: "inherit" });
  const status = await child.exited;
  if (status !== 0) throw new CliError(`Command failed: ${argv.join(" ")}`);
  console.log(`\n[quickgui] Wrote ${publicKeyPath}`);
  console.log(`[quickgui] Wrote ${secretKeyPath}`);
  console.log(
    "\nKeep the secret key out of version control. Pass its path through " +
      "`updates.minisignSecretKey` or QUICKGUI_MINISIGN_SECRET_KEY, and embed the public key in " +
      "your application's UpdateClient.",
  );
  return 0;
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
    updateManifest: command.updateManifest || (config.updates?.manifest ?? false),
    macAppStore: command.macAppStore,
    ...(command.outDir ? { outDir: command.outDir } : {}),
    ...(command.signingIdentity ? { signingIdentity: command.signingIdentity } : {}),
    ...(command.updateBaseUrl ? { updateBaseUrl: command.updateBaseUrl } : {}),
    ...(command.notarizationProfile
      ? { notarization: { keychainProfile: command.notarizationProfile } }
      : {}),
  });
  console.log(`[quickgui] Created ${result.artifactPath}`);
  if (result.dmgPath) console.log(`[quickgui] Created ${result.dmgPath}`);
  for (const path of result.packagePaths ?? []) console.log(`[quickgui] Created ${path}`);
  if (result.updateArtifactPath) {
    console.log(`[quickgui] Signed ${result.updateArtifactPath}`);
  }
  if (result.manifestPath) console.log(`[quickgui] Created ${result.manifestPath}`);
  for (const note of result.notes ?? []) console.log(`[quickgui] ${note}`);
  return 0;
}

function helpText(topic?: "init" | "dev" | "build" | "keygen"): string {
  if (topic === "keygen") {
    return `Usage: quickgui keygen [options]

Create a Minisign key pair for signing application updates.

Options:
  --out-dir <directory>      Where to write the key pair (default: .)
  --force                    Overwrite an existing key pair
  --password                 Encrypt the secret key with a password
  -h, --help                 Show this help`;
  }
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
  --notarize <profile>       Notary Keychain profile for the macOS DMG
  --mas                      Sign for the Mac App Store and build a .pkg
  --update-manifest          Sign the update artifact and write latest.json
  --update-base-url <url>    Publication URL for the signed update artifact
  -h, --help                 Show this help`;
  }
  return `QuickGUI CLI ${CLI_VERSION}

Usage: quickgui <command> [options]

Commands:
  init [directory]           Create a Solid-powered project
  dev                        Run a native app with source reload
  build                      Package a production application
  keygen                     Create a Minisign update signing key pair

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
