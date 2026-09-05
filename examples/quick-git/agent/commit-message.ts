/**
 * Commit-message generation with a local coding agent CLI.
 *
 * The app never talks to a model API itself: it hands the staged diff to whichever agent CLI the
 * user already has (`codex exec` or `claude -p`), non-interactively, with tools disabled or the
 * sandbox read-only, and parses the single message that comes back. Prompt construction, output
 * bounding, and answer parsing are pure so they can be tested without either CLI installed.
 */

import { constants } from "node:fs";
import { access, mkdtemp, readFile, rm } from "node:fs/promises";
import { homedir, tmpdir } from "node:os";
import { join } from "node:path";

export type AgentId = "codex" | "claude";

export interface AgentDefinition {
  id: AgentId;
  label: string;
  command: string;
  description: string;
}

export const AGENTS: readonly AgentDefinition[] = [
  { id: "codex", label: "Codex", command: "codex", description: "OpenAI Codex CLI (codex exec)" },
  { id: "claude", label: "Claude", command: "claude", description: "Claude Code (claude -p)" },
];

export interface AvailableAgent extends AgentDefinition {
  executable: string;
}

/** Find the agent CLIs on this machine, searching the login-shell locations a GUI app lacks. */
export async function detectAgents(env: Readonly<Record<string, string | undefined>> = process.env): Promise<AvailableAgent[]> {
  const found: AvailableAgent[] = [];
  for (const agent of AGENTS) {
    const executable = await findExecutable(agent.command, env);
    if (executable) found.push({ ...agent, executable });
  }
  return found;
}

export async function findExecutable(
  command: string,
  env: Readonly<Record<string, string | undefined>> = process.env,
): Promise<string | undefined> {
  const home = env.HOME ?? homedir();
  const directories: string[] = [];
  for (const directory of (env.PATH ?? "").split(":")) if (directory) directories.push(directory);
  directories.push(
    join(home, ".local", "bin"),
    join(home, ".bun", "bin"),
    join(home, ".cargo", "bin"),
    join(home, ".local", "share", "mise", "shims"),
    join(home, ".volta", "bin"),
    join(home, ".npm-global", "bin"),
    "/opt/homebrew/bin",
    "/usr/local/bin",
    "/usr/bin",
  );
  for (const directory of new Set(directories)) {
    const candidate = join(directory, command);
    try {
      await access(candidate, constants.X_OK);
      return candidate;
    } catch {
      // Keep looking.
    }
  }
  return undefined;
}

export interface CommitMessageRequest {
  repositoryName: string;
  branch?: string;
  /** `git diff --cached` (or the working-tree diff when nothing is staged). */
  diff: string;
  /** One line per file: `M src/app.ts`, `A docs/x.md`. */
  files: readonly string[];
  /** Recent subjects, newest first, so the message matches the repository's conventions. */
  recentSubjects: readonly string[];
  /** The message being amended, if any. */
  amending?: string;
  /** Optional instruction from the user, such as "mention the ticket number". */
  hint?: string;
}

/** Characters of diff kept in the prompt. Beyond this, files are cut whole and summarized. */
export const MAX_PROMPT_DIFF_CHARACTERS = 60_000;
export const MAX_PROMPT_FILES = 200;
export const MAX_RECENT_SUBJECTS = 15;

export function buildPrompt(request: CommitMessageRequest): string {
  const lines: string[] = [];
  lines.push(
    "You write git commit messages. Reply with only the commit message and nothing else: no preamble, no code fences, no quotes, no explanation.",
    "Rules: a subject line in the imperative mood of at most 72 characters; then, only when it adds real information, a blank line and a body wrapped at 72 columns that says what changed and why. Never restate the diff line by line. Do not add a sign-off or trailers.",
    "",
    `Repository: ${request.repositoryName}${request.branch ? ` (branch ${request.branch})` : ""}.`,
  );
  const subjects = request.recentSubjects.slice(0, MAX_RECENT_SUBJECTS);
  if (subjects.length > 0) {
    lines.push("", "Recent commit subjects, newest first. Match their conventions (prefixes, tense, capitalization, punctuation):");
    for (const subject of subjects) lines.push(`- ${subject}`);
  }
  if (request.amending) {
    lines.push("", "This commit amends the previous one, whose message was:", "", indent(request.amending));
  }
  if (request.hint?.trim()) {
    lines.push("", `Instruction from the author: ${request.hint.trim()}`);
  }
  const files = request.files.slice(0, MAX_PROMPT_FILES);
  lines.push("", `Files changed (${request.files.length}):`);
  for (const file of files) lines.push(`- ${file}`);
  if (request.files.length > files.length) lines.push(`- … and ${request.files.length - files.length} more`);
  const { text, omitted } = boundDiff(request.diff, MAX_PROMPT_DIFF_CHARACTERS);
  lines.push("", "Diff:", "```diff", text.replace(/\n$/, ""), "```");
  if (omitted > 0) lines.push("", `(${omitted} more changed ${omitted === 1 ? "file was" : "files were"} left out of the diff for length; the file list above is complete.)`);
  return `${lines.join("\n")}\n`;
}

function indent(text: string): string {
  return text
    .split("\n")
    .map((line) => `    ${line}`)
    .join("\n");
}

/** Keep whole files from the front of the diff until the budget runs out. */
export function boundDiff(diff: string, budget: number): { text: string; omitted: number } {
  if (diff.length <= budget) return { text: diff, omitted: 0 };
  const files = splitDiffFiles(diff);
  let text = "";
  let kept = 0;
  for (const file of files) {
    if (text.length + file.length > budget) break;
    text += file;
    kept += 1;
  }
  if (kept === 0 && files.length > 0) {
    // One file alone exceeds the budget: keep its head.
    text = `${files[0]!.slice(0, budget)}\n… (diff truncated)\n`;
    kept = 1;
  }
  return { text, omitted: files.length - kept };
}

function splitDiffFiles(diff: string): string[] {
  const files: string[] = [];
  let current = "";
  for (const line of diff.split(/(?<=\n)/)) {
    if (line.startsWith("diff --git ") && current) {
      files.push(current);
      current = "";
    }
    current += line;
  }
  if (current) files.push(current);
  return files;
}

export interface GeneratedMessage {
  subject: string;
  body: string;
  raw: string;
  agent: AgentId;
  durationMs: number;
}

/** Turn whatever the agent printed into a subject and body. */
export function parseMessage(raw: string): { subject: string; body: string } | undefined {
  let text = raw.replace(/\r\n/g, "\n").trim();
  // Strip a fenced block if the whole answer is one.
  const fence = /^```[a-z]*\n([\s\S]*?)\n```$/.exec(text);
  if (fence) text = fence[1]!.trim();
  // Strip labels and wrapping quotes some models add.
  text = text.replace(/^(?:commit message|message|subject)\s*:\s*/i, "");
  if (/^["“].*["”]$/s.test(text) && !text.includes("\n")) text = text.slice(1, -1);
  const lines = text.split("\n");
  while (lines.length > 0 && lines[0]!.trim() === "") lines.shift();
  if (lines.length === 0) return undefined;
  const subject = lines[0]!.trim().replace(/^["'`]|["'`]$/g, "");
  const body = lines
    .slice(1)
    .join("\n")
    .replace(/^\n+/, "")
    .replace(/\s+$/, "");
  if (!subject) return undefined;
  return { subject, body };
}

export interface ProcessRun {
  stdout: string;
  stderr: string;
  exitCode: number;
}

export type ProcessRunner = (
  command: readonly string[],
  options: { cwd: string; stdin: string; signal?: AbortSignal; timeoutMs: number },
) => Promise<ProcessRun>;

export class AgentError extends Error {
  readonly agent: AgentId;
  readonly aborted: boolean;
  constructor(message: string, agent: AgentId, aborted = false) {
    super(message);
    this.name = "AgentError";
    this.agent = agent;
    this.aborted = aborted;
  }
}

export const AGENT_TIMEOUT_MS = 120_000;

/** The exact non-interactive invocation for each agent; `outputFile` receives Codex's answer. */
export function agentCommand(agent: AvailableAgent, cwd: string, outputFile: string): string[] {
  switch (agent.id) {
    case "codex":
      return [
        agent.executable,
        "exec",
        "--skip-git-repo-check",
        "--sandbox",
        "read-only",
        "--color",
        "never",
        "--ephemeral",
        "-C",
        cwd,
        "-o",
        outputFile,
        "-",
      ];
    case "claude":
      return [
        agent.executable,
        "-p",
        "--output-format",
        "text",
        "--tools",
        "",
        "--no-session-persistence",
        "--disable-slash-commands",
      ];
  }
}

export async function generateCommitMessage(
  agent: AvailableAgent,
  request: CommitMessageRequest,
  cwd: string,
  options: { signal?: AbortSignal; run?: ProcessRunner } = {},
): Promise<GeneratedMessage> {
  const run = options.run ?? spawnProcess;
  const started = Date.now();
  const scratch = await mkdtemp(join(tmpdir(), "quick-git-agent-"));
  const outputFile = join(scratch, "message.txt");
  try {
    const prompt = buildPrompt(request);
    const result = await run(agentCommand(agent, cwd, outputFile), {
      cwd,
      stdin: prompt,
      ...(options.signal ? { signal: options.signal } : {}),
      timeoutMs: AGENT_TIMEOUT_MS,
    });
    if (options.signal?.aborted) throw new AgentError("cancelled", agent.id, true);
    let raw = result.stdout;
    if (agent.id === "codex") {
      try {
        raw = await readFile(outputFile, "utf8");
      } catch {
        // Fall back to stdout when the CLI did not write the file.
      }
    }
    if (result.exitCode !== 0 && !raw.trim()) {
      throw new AgentError(summarizeFailure(agent, result), agent.id);
    }
    const parsed = parseMessage(raw);
    if (!parsed) throw new AgentError(`${agent.label} returned an empty message`, agent.id);
    return { ...parsed, raw, agent: agent.id, durationMs: Date.now() - started };
  } finally {
    await rm(scratch, { recursive: true, force: true });
  }
}

function summarizeFailure(agent: AvailableAgent, result: ProcessRun): string {
  const lines = `${result.stderr}\n${result.stdout}`
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
  const last = lines.at(-1);
  return last ? `${agent.label}: ${last.slice(0, 200)}` : `${agent.label} exited with code ${result.exitCode}`;
}

const spawnProcess: ProcessRunner = async (command, options) => {
  const child = Bun.spawn([...command], {
    cwd: options.cwd,
    env: { ...process.env, NO_COLOR: "1", TERM: "dumb" },
    stdin: "pipe",
    stdout: "pipe",
    stderr: "pipe",
  });
  const kill = () => {
    try {
      child.kill();
    } catch {
      // Already exited.
    }
  };
  options.signal?.addEventListener("abort", kill, { once: true });
  const timer = setTimeout(kill, options.timeoutMs);
  try {
    child.stdin.write(options.stdin);
    await child.stdin.end();
  } catch {
    // The process may exit before consuming its input.
  }
  try {
    const [stdout, stderr, exitCode] = await Promise.all([
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
      child.exited,
    ]);
    return { stdout, stderr, exitCode };
  } finally {
    clearTimeout(timer);
    options.signal?.removeEventListener("abort", kill);
  }
};
