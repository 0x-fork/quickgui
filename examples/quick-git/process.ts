import { spawn, type ChildProcess } from "node:child_process";
import { Buffer } from "node:buffer";
import { accessSync, constants, mkdtempSync, rmSync } from "node:fs";
import { writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

export interface ProcessOptions {
  cwd: string;
  stdin?: string | Uint8Array;
  env?: Readonly<Record<string, string>>;
  signal?: AbortSignal | undefined;
  timeoutMs?: number;
  maxOutputBytes?: number;
}

export interface ProcessResult {
  stdout: Uint8Array;
  stderr: string;
  exitCode: number;
  truncated: boolean;
  aborted: boolean;
  timedOut: boolean;
}

export function inheritedEnvironment(): Record<string, string> {
  const result: Record<string, string> = {};
  for (const name of Object.keys(process.env)) {
    const value = process.env[name];
    if (value !== undefined) result[name] = value;
  }
  return result;
}

export function executableExists(path: string): boolean {
  try { accessSync(path, constants.X_OK); return true; } catch { return false; }
}

/** Bounded asynchronous child process, including stdin on the static runtime. */
export async function runProcess(command: readonly string[], options: ProcessOptions): Promise<ProcessResult> {
  if (command.length === 0) throw new Error("A process executable is required");
  let scratch = "";
  try {
    if (options.signal?.aborted) return cancelledResult();
    let executable = command[0]!;
    let args = command.slice(1);
    if (options.stdin !== undefined) {
      // scriptc currently supports async stdout/stderr pipes but no writable stdin pipe.
      // A private input file plus a fixed shell program preserves binary stdin and exact argv.
      // exec replaces the shell, so cancellation still addresses the actual child process.
      scratch = mkdtempSync(join(tmpdir(), "quick-git-input-"));
      const input = join(scratch, "stdin");
      await writeFile(input, options.stdin);
      args = ["-c", 'input=$1; shift; exec "$@" < "$input"', "quick-git", input, ...command];
      executable = "/bin/sh";
    }
    if (options.signal?.aborted) return cancelledResult();
    return await collectProcess(executable, args, options);
  } finally {
    if (scratch) rmSync(scratch, { recursive: true, force: true });
  }
}

function cancelledResult(): ProcessResult {
  return { stdout: new Uint8Array(0), stderr: "", exitCode: -1, truncated: false, aborted: true, timedOut: false };
}

async function collectProcess(executable: string, args: string[], options: ProcessOptions): Promise<ProcessResult> {
  const environment = inheritedEnvironment();
  const overrides = options.env;
  if (overrides !== undefined) for (const name of Object.keys(overrides)) environment[name] = overrides[name]!;
  const child: ChildProcess = spawn(executable, args, { cwd: options.cwd, env: environment, stdio: ["ignore", "pipe", "pipe"] });
  const chunks: Uint8Array[] = [];
  let bytes = 0;
  let stderr = "";
  let stderrBytes = 0;
  let truncated = false;
  let aborted = false;
  let timedOut = false;
  const maximum = Math.max(0, options.maxOutputBytes ?? 64 * 1024 * 1024);
  const kill = () => { try { child.kill("SIGKILL"); } catch {} };
  const onAbort = () => { aborted = true; kill(); };
  const signal = options.signal;
  if (signal !== undefined) signal.addEventListener("abort", onAbort, { once: true });
  const timer = setTimeout(() => { timedOut = true; kill(); }, options.timeoutMs ?? 60_000);
  try {
    const exitCode = await new Promise<number>((resolve, reject) => {
      let ended = 0;
      let exited = false;
      let code = -1;
      const complete = () => { if (exited && ended === 2) resolve(code); };
      const stdout = child.stdout;
      const errors = child.stderr;
      if (stdout !== null) {
        stdout.on("data", (chunk: Buffer) => {
          const room = Math.max(0, maximum - bytes);
          const kept = Math.min(room, chunk.byteLength);
          if (kept > 0) { chunks.push(new Uint8Array(chunk.subarray(0, kept))); bytes += kept; }
          if (kept < chunk.byteLength) { truncated = true; kill(); }
        });
        stdout.on("end", () => { ended += 1; complete(); });
      } else ended += 1;
      if (errors !== null) {
        errors.on("data", (chunk: Buffer) => {
          // Keep diagnostics bounded independently; consuming the rest avoids a blocked pipe.
          const room = Math.max(0, 1024 * 1024 - stderrBytes);
          const kept = Math.min(room, chunk.byteLength);
          stderr += chunk.subarray(0, kept).toString("utf8");
          stderrBytes += kept;
        });
        errors.on("end", () => { ended += 1; complete(); });
      } else ended += 1;
      child.on("error", (error: Error) => { reject(error); });
      child.on("exit", (status: number | null) => { code = status ?? -1; exited = true; complete(); });
      if (signal !== undefined && signal.aborted) onAbort();
    });
    const stdout = new Uint8Array(bytes);
    let offset = 0;
    for (const chunk of chunks) { stdout.set(chunk, offset); offset += chunk.byteLength; }
    return { stdout, stderr, exitCode, truncated, aborted, timedOut };
  } finally {
    clearTimeout(timer);
    if (signal !== undefined) signal.removeEventListener("abort", onAbort);
  }
}
