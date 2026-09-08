import { join, resolve } from "node:path";
import { existsSync } from "node:fs";
import { CliError } from "./error.ts";
import { loadConfig } from "./config.ts";

/** The compiler runs on the host; its overlay is type-checked for the application's target. */
export async function prepareGoWorkspace(
  project: string,
  patterns: string[],
  options: {
    tests?: boolean;
    tags?: string[];
    env?: Record<string, string | undefined>;
  } = {},
): Promise<string | undefined> {
  const env = options.env ?? process.env;
  const module = Bun.spawn(["go", "list", "-m", "-json", "github.com/egoist/quickgui/go"], {
    cwd: project,
    env: { ...env, CGO_ENABLED: "0" },
    stdin: "ignore",
    stdout: "pipe",
    stderr: "pipe",
  });
  const [moduleStatus, , moduleError] = await Promise.all([
    module.exited,
    new Response(module.stdout).text(),
    new Response(module.stderr).text(),
  ]);
  if (moduleStatus !== 0) {
    if (moduleError.includes("not a known dependency")) return;
    throw new CliError(`Cannot resolve the Go SDK\n${moduleError.trim()}`);
  }
  const child = Bun.spawn(
    [
      "go",
      "run",
      "github.com/egoist/quickgui/go/cmd/quickguic",
      "-project",
      resolve(project),
      "-out",
      join(resolve(project), ".quickgui/go-sources"),
      ...(options.tests ? ["-tests"] : []),
      ...(options.tags?.length ? ["-tags", options.tags.join(",")] : []),
      ...(env.GOOS ? ["-goos", env.GOOS] : []),
      ...(env.GOARCH ? ["-goarch", env.GOARCH] : []),
      ...patterns,
    ],
    {
      cwd: project,
      env: { ...env, CGO_ENABLED: "0", GOOS: undefined, GOARCH: undefined },
      stdin: "ignore",
      stdout: "pipe",
      stderr: "pipe",
    },
  );
  const [status, stdout, stderr] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);
  if (status !== 0)
    throw new CliError(`Go view compilation failed\n${stderr.trim() || stdout.trim()}`);
  const overlay = stdout.trim();
  if (!existsSync(overlay)) throw new CliError("Go view compiler did not write its source overlay");
  return overlay;
}

export async function runGo(
  project: string,
  command: "check" | "test",
  release = false,
): Promise<number> {
  const configured = ["quickgui.toml", "quickgui.config.ts"].some((file) =>
    existsSync(join(project, file)),
  );
  const tags = configured ? (await loadConfig(project)).native.tags : [];
  const overlay = await prepareGoWorkspace(project, ["./..."], { tests: true, tags });
  const child = Bun.spawn(
    [
      "go",
      "test",
      ...(overlay ? ["-overlay", overlay] : []),
      ...(tags.length ? ["-tags", tags.join(",")] : []),
      ...(command === "check"
        ? ["-run", "^$", "-exec", process.platform === "win32" ? "cmd /c exit 0" : "true"]
        : []),
      ...(release ? ["-trimpath"] : []),
      "./...",
    ],
    {
      cwd: project,
      env: { ...process.env, CGO_ENABLED: "0" },
      stdin: "inherit",
      stdout: "inherit",
      stderr: "inherit",
    },
  );
  return child.exited;
}
