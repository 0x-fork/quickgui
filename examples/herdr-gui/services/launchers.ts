import { constants } from "node:fs";
import { access } from "node:fs/promises";
import { join } from "node:path";

import type { AgentLauncher } from "../model.ts";

export const launcherDefinitions: readonly Omit<
  AgentLauncher,
  "installed" | "executable"
>[] = [
  {
    id: "codex",
    label: "Codex",
    mark: "C",
    description: "OpenAI Codex CLI",
    command: "codex",
    argumentsFor: (prompt) => (prompt ? [prompt] : []),
  },
  {
    id: "claude",
    label: "Claude",
    mark: "A",
    description: "Anthropic Claude Code",
    command: "claude",
    argumentsFor: (prompt) => (prompt ? [prompt] : []),
  },
  {
    id: "opencode",
    label: "OpenCode",
    mark: "O",
    description: "OpenCode terminal agent",
    command: "opencode",
    argumentsFor: (prompt) => (prompt ? ["--prompt", prompt] : []),
  },
];

export function loginShell(): string {
  return process.env.SHELL || "/bin/zsh";
}

export async function captureShellEnvironment(): Promise<
  Record<string, string>
> {
  const inherited = Object.fromEntries(
    Object.entries(process.env).filter(
      (entry): entry is [string, string] => typeof entry[1] === "string",
    ),
  );
  const shell = inherited.SHELL || "/bin/zsh";
  try {
    const child = Bun.spawn([shell, "-i", "-l", "-c", "/usr/bin/env -0"], {
      env: {
        ...inherited,
        DISABLE_AUTO_UPDATE: "true",
        DISABLE_UPDATE_PROMPT: "true",
        ZSH_DISABLE_COMPFIX: "true",
      },
      stdin: "ignore",
      stdout: "pipe",
      stderr: "ignore",
    });
    const timeout = setTimeout(() => child.kill(), 4_000);
    const [bytes, exitCode] = await Promise.all([
      new Response(child.stdout).arrayBuffer(),
      child.exited,
    ]);
    clearTimeout(timeout);
    if (exitCode !== 0) return inherited;
    const environment = { ...inherited };
    for (const entry of new TextDecoder().decode(bytes).split("\0")) {
      const separator = entry.indexOf("=");
      if (separator <= 0) continue;
      environment[entry.slice(0, separator)] = entry.slice(separator + 1);
    }
    return environment;
  } catch {
    return inherited;
  }
}

export async function resolveLaunchers(
  environment: Readonly<Record<string, string>>,
  homePath: string,
): Promise<AgentLauncher[]> {
  return Promise.all(
    launcherDefinitions.map(async (definition): Promise<AgentLauncher> => {
      const executable = await findExecutable(
        definition.command,
        environment,
        homePath,
      );
      return {
        ...definition,
        ...(executable ? { executable } : {}),
        installed: executable !== undefined,
      };
    }),
  );
}

async function findExecutable(
  command: string,
  environment: Readonly<Record<string, string>>,
  homePath: string,
): Promise<string | undefined> {
  const directories = new Set<string>();
  for (const path of [environment.PATH, process.env.PATH]) {
    for (const directory of (path ?? "").split(":")) {
      if (directory) directories.add(directory);
    }
  }
  for (const directory of [
    join(homePath, ".local/bin"),
    join(homePath, ".bun/bin"),
    join(homePath, ".cargo/bin"),
    join(homePath, ".local/share/mise/shims"),
    join(homePath, ".volta/bin"),
    "/opt/homebrew/bin",
    "/usr/local/bin",
    "/usr/bin",
    "/bin",
  ]) {
    directories.add(directory);
  }
  for (const directory of directories) {
    const candidate = join(directory, command);
    try {
      await access(candidate, constants.X_OK);
      return candidate;
    } catch {
      // Continue through the bounded login-shell search path.
    }
  }
  return undefined;
}
