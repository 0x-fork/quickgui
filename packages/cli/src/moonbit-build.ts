/** MoonBit native builds reuse the same core and extension shared libraries as Go. */
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  realpathSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, isAbsolute, join, relative, resolve } from "node:path";
import type { NativeCompileOptions } from "./native-build.ts";
import { extensionManifests, type ExtensionManifest } from "./extensions.ts";
import { CliError } from "./error.ts";
import { hostTarget, targetInfo, type QuickGuiTarget } from "./targets.ts";
import { updaterMetadata } from "./packaging/appcast.ts";
import { prepareMoonbitWorkspace, renderMoonbitOutput, stagedEntry } from "./moonbit/workspace.ts";

/** Direct native code generation avoids recompiling generated application C on every edit. */
export function moonbitBuildEnvironment(
  mode: NativeCompileOptions["mode"],
  target: QuickGuiTarget,
  environment: NodeJS.ProcessEnv = process.env,
): NodeJS.ProcessEnv {
  const nativeDebug =
    mode === "development" &&
    ["darwin-arm64", "linux-x64", "windows-x64"].includes(target) &&
    environment.MOONBIT_NEW_NATIVE !== "0";
  return { ...environment, MOONBIT_NEW_NATIVE: nativeDebug ? "1" : "0" };
}

export function moonbitBuildPlan(options: NativeCompileOptions) {
  if (options.target !== hostTarget())
    throw new CliError(
      "MoonBit native builds currently require the target platform and architecture to match the build host",
    );
  const { config } = options;
  const entry = relative(config.projectRoot, config.entry).replaceAll("\\", "/") || ".";
  if (entry.startsWith("../") || isAbsolute(entry))
    throw new CliError("MoonBit entry must be a package inside the project directory");
  const profile = options.mode === "production" ? "release" : "debug";
  const targetDir = join(config.projectRoot, ".quickgui", "moonbit");
  return {
    argv: [
      "moon",
      "build",
      config.entry,
      "--target",
      "native",
      `--${profile}`,
      "--target-dir",
      targetDir,
    ],
    env: moonbitBuildEnvironment(options.mode, options.target),
    targetDir,
    profile,
    entry: entry === "." ? "" : entry,
  };
}

export function moonbitExtensions(directories: string[]): ExtensionManifest[] {
  for (const directory of directories) {
    if (!existsSync(directory) || !statSync(directory).isDirectory())
      throw new CliError(`Expected an extension directory: ${directory}`);
    const manifest = join(directory, "quickgui.extension.json");
    if (!existsSync(manifest) || !statSync(manifest).isFile())
      throw new CliError(
        `Expected a quickgui.extension.json file in extension directory: ${directory}`,
      );
  }
  return extensionManifests(directories.map((directory) => ({ Dir: directory })));
}

export function moonbitModuleName(project: string): string {
  const modern = join(project, "moon.mod");
  const legacy = join(project, "moon.mod.json");
  let name: unknown;
  if (existsSync(modern)) {
    const match = /^\s*name\s*=\s*("(?:[^"\\]|\\.)*")\s*(?:\/\/[^\n]*)?$/m.exec(
      readFileSync(modern, "utf8"),
    );
    if (match) name = JSON.parse(match[1]!);
  } else if (existsSync(legacy)) name = JSON.parse(readFileSync(legacy, "utf8")).name;
  if (typeof name !== "string" || !/^[A-Za-z0-9_-]+(?:\/[A-Za-z0-9_.-]+)+$/.test(name))
    throw new CliError("MoonBit project needs a module name in moon.mod (or moon.mod.json)");
  return name;
}

export function requireMoonbitExecutable(entry: string): void {
  const modern = join(entry, "moon.pkg");
  const legacy = join(entry, "moon.pkg.json");
  const main = existsSync(modern)
    ? /\bpkgtype\s*\(\s*kind\s*:\s*"executable"\s*,?\s*\)/.test(
        readFileSync(modern, "utf8").replace(/\/\*[\s\S]*?\*\/|\/\/[^\n]*/g, ""),
      )
    : existsSync(legacy) && JSON.parse(readFileSync(legacy, "utf8"))["is-main"] === true;
  if (!main)
    throw new CliError(
      `MoonBit entry must declare pkgtype(kind: "executable") in moon.pkg: ${entry}`,
    );
}

/** Match the requested module/package in Moon's build manifest; never select an arbitrary executable. */
export function moonbitExecutable(
  manifest: unknown,
  module: string,
  entry: string,
  buildDir: string,
): string {
  const packages = (manifest as { packages?: unknown } | null)?.packages;
  if (!Array.isArray(packages))
    throw new CliError(
      "Unsupported MoonBit build manifest; update the QuickGUI CLI/toolchain together",
    );
  const matches = packages.filter((pkg) => pkg?.root === module && pkg?.rel === entry);
  if (
    matches.length !== 1 ||
    typeof matches[0]?.artifact !== "string" ||
    !matches[0].artifact.endsWith(".mi")
  )
    throw new CliError(
      `MoonBit did not report the requested executable package ${module}/${entry}`,
    );
  const artifact = resolve(matches[0].artifact.slice(0, -3) + ".exe");
  const executable = existsSync(artifact) ? realpathSync(artifact) : artifact;
  const directory = existsSync(buildDir) ? realpathSync(buildDir) : resolve(buildDir);
  const path = relative(directory, executable);
  if (path.startsWith("..") || isAbsolute(path))
    throw new CliError("MoonBit artifact is outside the selected build directory");
  return executable;
}

export async function compileMoonbitApplication(
  options: NativeCompileOptions,
  extensions: ExtensionManifest[],
): Promise<string> {
  if (!Bun.which("moon", { PATH: process.env.PATH ?? "" }))
    throw new CliError("MoonBit's moon command and a native C compiler are required on PATH");
  const plan = moonbitBuildPlan(options);
  const module = moonbitModuleName(options.config.projectRoot);
  // Moon leaves old outputs in its incremental cache when a main becomes a library.
  requireMoonbitExecutable(options.config.entry);
  const workspace = await prepareMoonbitWorkspace(options.config.projectRoot);
  plan.argv[2] = stagedEntry(workspace, options.config.projectRoot, options.config.entry);
  plan.argv.push("--output-json");
  const child = Bun.spawn(plan.argv, {
    cwd: workspace.project,
    env: plan.env,
    stdin: "ignore",
    stdout: "pipe",
    stderr: "pipe",
  });
  const [code, stdout, stderr] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);
  if (code !== 0)
    throw new CliError(
      `MoonBit compilation failed\n${renderMoonbitOutput([stdout.trim(), stderr.trim()].filter(Boolean).join("\n"), workspace.sources, workspace.project)}`,
    );
  const buildDir = join(plan.targetDir, "native", plan.profile, "build");
  const manifest = join(buildDir, "all_pkgs.json");
  if (!existsSync(manifest))
    throw new CliError("MoonBit did not produce its native build manifest");
  const executable = moonbitExecutable(
    JSON.parse(readFileSync(manifest, "utf8")),
    module,
    plan.entry,
    buildDir,
  );
  if (!existsSync(executable))
    throw new CliError(
      `MoonBit entry must be an executable package (pkgtype(kind: "executable")): ${options.config.entry}`,
    );
  copyFileSync(executable, options.executablePath);
  chmodSync(options.executablePath, 0o755);
  const platform = targetInfo(options.target).platform;
  const metadata = join(
    dirname(options.executablePath),
    ...(platform === "darwin" ? ["..", "Resources"] : []),
    "quickgui-moonbit.json",
  );
  const config = options.config;
  mkdirSync(dirname(metadata), { recursive: true });
  writeFileSync(
    metadata,
    JSON.stringify({
      name: config.name,
      version: config.version,
      identifier: options.mode === "development" ? `${config.identifier}.dev` : config.identifier,
      fonts: options.fonts,
      extensions: extensions.map(({ name, version }) => ({ name, version })),
      ...(extensions.some(
        (extension) =>
          extension.name === "updater" && extension.package === "@quickgui/native-updater",
      )
        ? { updater: updaterMetadata(config, options.target, options.mode) }
        : {}),
    }),
  );
  return metadata;
}

/** Check, test, and native smoke runs must use exactly the same view transform as application builds. */
export async function runMoonbit(
  project: string,
  args: string[],
  mode: NativeCompileOptions["mode"] = "development",
  environment: NodeJS.ProcessEnv = process.env,
  timeout?: number,
): Promise<number> {
  project = resolve(project);
  const workspace = await prepareMoonbitWorkspace(project);
  const child = Bun.spawn(
    [
      "moon",
      ...args,
      "--target",
      "native",
      mode === "development" ? "--debug" : "--release",
      "--target-dir",
      join(project, ".quickgui/moonbit"),
      "--output-json",
    ],
    {
      cwd: workspace.project,
      env: moonbitBuildEnvironment(mode, hostTarget(), environment),
      stdin: "ignore",
      stdout: "pipe",
      stderr: "pipe",
    },
  );
  const timer = timeout === undefined ? undefined : setTimeout(() => child.kill(), timeout);
  try {
    const [status, stdout, stderr] = await Promise.all([
      child.exited,
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
    ]);
    if (stdout)
      process.stdout.write(renderMoonbitOutput(stdout, workspace.sources, workspace.project));
    if (stderr)
      process.stderr.write(renderMoonbitOutput(stderr, workspace.sources, workspace.project));
    return status;
  } finally {
    clearTimeout(timer);
  }
}
