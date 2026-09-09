/**
 * Bounded git process runner.
 *
 * Every git invocation in the app goes through one `GitRunner`: it caps how many git processes
 * run at once, lets interactive work (status, the diff the user is looking at) jump ahead of
 * background work (history, refs), kills a process when its caller aborts or its deadline
 * passes, bounds the bytes it will buffer, and turns a failure into a typed `GitError` that
 * carries the command, the exit code, and git's own stderr. Nothing here touches the UI.
 */

export type GitPriority = "interactive" | "background";

export interface GitCommandOptions {
  /** Working directory the command runs in. */
  cwd: string;
  /** Aborting kills the process; the promise rejects with an aborted `GitError`. */
  signal?: AbortSignal;
  /** Bytes written to the process's stdin before it is closed. */
  stdin?: string | Uint8Array;
  /** Wall-clock deadline. Defaults to sixty seconds; network commands pass more. */
  timeoutMs?: number;
  /** Stdout bytes kept before the process is killed and the output marked truncated. */
  maxOutputBytes?: number;
  /** Extra environment on top of the runner's own. */
  env?: Readonly<Record<string, string>>;
  /** Exit codes that are a valid answer rather than a failure (`diff --no-index` exits 1). */
  allowExitCodes?: readonly number[];
  priority?: GitPriority;
}

export interface GitResult {
  stdout: Uint8Array;
  stderr: string;
  exitCode: number;
  /** Whether stdout hit `maxOutputBytes` and the process was killed. */
  truncated: boolean;
}

export class GitError extends Error {
  readonly command: readonly string[];
  readonly cwd: string;
  readonly exitCode: number | null;
  readonly stderr: string;
  readonly aborted: boolean;
  readonly timedOut: boolean;

  constructor(
    message: string,
    details: {
      command: readonly string[];
      cwd: string;
      exitCode: number | null;
      stderr: string;
      aborted?: boolean;
      timedOut?: boolean;
    },
  ) {
    super(message);
    this.name = "GitError";
    this.command = details.command;
    this.cwd = details.cwd;
    this.exitCode = details.exitCode;
    this.stderr = details.stderr;
    this.aborted = details.aborted ?? false;
    this.timedOut = details.timedOut ?? false;
  }

  /** The most useful single line for a toast: git's own last stderr line, or the exit code. */
  get summary(): string {
    const lines = this.stderr
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => line.length > 0 && !line.startsWith("hint:"));
    const last = lines.at(-1);
    if (last) return last.replace(/^(fatal|error):\s*/, "");
    if (this.timedOut) return "git timed out";
    if (this.aborted) return "git was cancelled";
    return `git exited with code ${this.exitCode ?? "unknown"}`;
  }
}

export const DEFAULT_GIT_TIMEOUT_MS = 60_000;
export const NETWORK_GIT_TIMEOUT_MS = 10 * 60_000;
export const DEFAULT_MAX_OUTPUT_BYTES = 64 * 1024 * 1024;

const decoder = new TextDecoder("utf-8", { fatal: false });

/** Decode git output as UTF-8, tolerating paths that are not. */
export function decodeOutput(bytes: Uint8Array): string {
  return decoder.decode(bytes);
}

interface QueuedCommand {
  priority: GitPriority;
  start: () => void;
}

export interface GitRunnerOptions {
  /** Path to the git executable. Defaults to the first `git` on `PATH`, then `/usr/bin/git`. */
  executable?: string;
  /** Maximum concurrently running git processes. */
  concurrency?: number;
  /** Extra environment applied to every command. */
  env?: Readonly<Record<string, string>>;
}

export class GitRunner {
  readonly executable: string;
  readonly concurrency: number;
  readonly #env: Record<string, string>;
  readonly #queue: QueuedCommand[] = [];
  #running = 0;
  #inFlight = 0;

  constructor(options: GitRunnerOptions = {}) {
    this.executable = options.executable ?? resolveGitExecutable();
    this.concurrency = Math.max(1, options.concurrency ?? 4);
    this.#env = {
      ...inheritedEnvironment(),
      // Never block on a credential or passphrase prompt: there is no terminal to answer it.
      GIT_TERMINAL_PROMPT: "0",
      // Read-only commands must not take the index lock the app's own writes need.
      GIT_OPTIONAL_LOCKS: "0",
      // Stable, parseable messages.
      LC_ALL: "C",
      ...(options.env ?? {}),
    };
  }

  /** Commands running plus commands waiting for a slot. */
  get pending(): number {
    return this.#inFlight;
  }

  async run(args: readonly string[], options: GitCommandOptions): Promise<GitResult> {
    if (options.signal?.aborted) {
      throw new GitError("git was cancelled before it started", {
        command: [this.executable, ...args],
        cwd: options.cwd,
        exitCode: null,
        stderr: "",
        aborted: true,
      });
    }
    this.#inFlight += 1;
    try {
      await this.#acquire(options.priority ?? "interactive", options.signal);
      try {
        return await this.#spawn(args, options);
      } finally {
        this.#release();
      }
    } finally {
      this.#inFlight -= 1;
    }
  }

  /** Run and decode stdout, throwing on any exit code not explicitly allowed. */
  async text(args: readonly string[], options: GitCommandOptions): Promise<string> {
    const result = await this.run(args, options);
    return decodeOutput(result.stdout);
  }

  #acquire(priority: GitPriority, signal: AbortSignal | undefined): Promise<void> {
    if (this.#running < this.concurrency && this.#queue.length === 0) {
      this.#running += 1;
      return Promise.resolve();
    }
    return new Promise((resolve, reject) => {
      const entry: QueuedCommand = {
        priority,
        start: () => {
          signal?.removeEventListener("abort", onAbort);
          this.#running += 1;
          resolve();
        },
      };
      const onAbort = () => {
        const index = this.#queue.indexOf(entry);
        if (index >= 0) this.#queue.splice(index, 1);
        reject(
          new GitError("git was cancelled while waiting for a slot", {
            command: [this.executable],
            cwd: "",
            exitCode: null,
            stderr: "",
            aborted: true,
          }),
        );
      };
      signal?.addEventListener("abort", onAbort, { once: true });
      // Interactive work goes ahead of every queued background command, behind earlier
      // interactive commands, so a diff the user asked for never waits behind history paging.
      const insertAt =
        priority === "interactive"
          ? this.#queue.findIndex((queued) => queued.priority === "background")
          : -1;
      if (insertAt >= 0) this.#queue.splice(insertAt, 0, entry);
      else this.#queue.push(entry);
    });
  }

  #release(): void {
    this.#running -= 1;
    const next = this.#queue.shift();
    if (next) next.start();
  }

  async #spawn(args: readonly string[], options: GitCommandOptions): Promise<GitResult> {
    const command = [
      this.executable,
      "-c",
      "core.quotepath=false",
      "-c",
      "color.ui=never",
      ...args,
    ];
    const maxOutputBytes = options.maxOutputBytes ?? DEFAULT_MAX_OUTPUT_BYTES;
    const timeoutMs = options.timeoutMs ?? DEFAULT_GIT_TIMEOUT_MS;
    let child: ReturnType<typeof Bun.spawn>;
    try {
      child = Bun.spawn(command, {
        cwd: options.cwd,
        env: { ...this.#env, ...(options.env ?? {}) },
        stdin: options.stdin === undefined ? "ignore" : "pipe",
        stdout: "pipe",
        stderr: "pipe",
      });
    } catch (error) {
      throw new GitError(
        `unable to start git: ${error instanceof Error ? error.message : String(error)}`,
        { command, cwd: options.cwd, exitCode: null, stderr: "" },
      );
    }

    let aborted = false;
    let timedOut = false;
    let truncated = false;
    const kill = () => {
      try {
        child.kill();
      } catch {
        // Already gone.
      }
    };
    const onAbort = () => {
      aborted = true;
      kill();
    };
    options.signal?.addEventListener("abort", onAbort, { once: true });
    const timer = setTimeout(() => {
      timedOut = true;
      kill();
    }, timeoutMs);

    if (options.stdin !== undefined && child.stdin && typeof child.stdin !== "number") {
      const writer = child.stdin;
      try {
        writer.write(options.stdin);
        await writer.end();
      } catch {
        // The process may have exited before reading all of its input; the exit code tells.
      }
    }

    const collectStdout = async (): Promise<Uint8Array> => {
      const chunks: Uint8Array[] = [];
      let total = 0;
      const stream = child.stdout as ReadableStream<Uint8Array>;
      const reader = stream.getReader();
      for (;;) {
        const { value, done } = await reader.read();
        if (done) break;
        if (total + value.byteLength > maxOutputBytes) {
          const room = Math.max(0, maxOutputBytes - total);
          if (room > 0) chunks.push(value.subarray(0, room));
          total += room;
          truncated = true;
          kill();
          break;
        }
        chunks.push(value);
        total += value.byteLength;
      }
      // Drain whatever is left so the process can exit.
      try {
        for (;;) {
          const { done } = await reader.read();
          if (done) break;
        }
      } catch {
        // Killed mid-read.
      }
      return concat(chunks, total);
    };

    try {
      const [stdout, stderrText, exitCode] = await Promise.all([
        collectStdout(),
        new Response(child.stderr as ReadableStream<Uint8Array>).text(),
        child.exited,
      ]);
      const allowed = options.allowExitCodes ?? [];
      if (aborted || timedOut) {
        throw new GitError(
          timedOut ? `git ${args[0] ?? ""} timed out` : `git ${args[0] ?? ""} was cancelled`,
          { command, cwd: options.cwd, exitCode, stderr: stderrText, aborted, timedOut },
        );
      }
      if (exitCode !== 0 && !allowed.includes(exitCode) && !truncated) {
        throw new GitError(
          `git ${args
            .filter((argument) => !argument.startsWith("-"))
            .slice(0, 2)
            .join(" ")} failed`,
          { command, cwd: options.cwd, exitCode, stderr: stderrText },
        );
      }
      return { stdout, stderr: stderrText, exitCode, truncated };
    } finally {
      clearTimeout(timer);
      options.signal?.removeEventListener("abort", onAbort);
    }
  }
}

function concat(chunks: Uint8Array[], total: number): Uint8Array {
  if (chunks.length === 1) return chunks[0]!;
  const out = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    out.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return out;
}

function inheritedEnvironment(): Record<string, string> {
  const inherited: Record<string, string> = {};
  for (const [key, value] of Object.entries(process.env)) {
    if (typeof value === "string") inherited[key] = value;
  }
  return inherited;
}

export function resolveGitExecutable(): string {
  const override = process.env.QUICK_GIT_EXECUTABLE?.trim();
  if (override) return override;
  const onPath = Bun.which("git");
  if (onPath) return onPath;
  for (const candidate of ["/opt/homebrew/bin/git", "/usr/local/bin/git", "/usr/bin/git"]) {
    if (Bun.which(candidate)) return candidate;
  }
  return "git";
}

/** Split NUL-terminated output into records, dropping the empty tail. */
export function splitNul(text: string): string[] {
  if (text.length === 0) return [];
  const parts = text.split("\0");
  if (parts.at(-1) === "") parts.pop();
  return parts;
}
