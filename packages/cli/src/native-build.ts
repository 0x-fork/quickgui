/**
 * Compile an application to a native executable: lower the project, write the FFI manifest that
 * binds the host library, and drive the scriptc compiler with a linker wrapper that makes the
 * Rust host own the process entry.
 */

import { chmodSync, existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

import type { ResolvedQuickGuiConfig } from "./config.ts";
import { CliError } from "./error.ts";
import { lowerProject, resolvePackageDir } from "./native-compiler.ts";
import { targetInfo, type QuickGuiTarget } from "./targets.ts";

export interface HostLibrary {
  archive: string;
  entry: string;
  frameworks: string[];
  libraries: string[];
  searchPaths: string[];
}

interface LinkRecipe {
  target: string;
  entry: string;
  frameworks: string[];
  libraries: string[];
  searchPaths: string[];
}

/** The macOS frameworks the host archive needs when no staged recipe is available. */
const macFrameworks = [
  "AppKit",
  "ApplicationServices",
  "AVFoundation",
  "Carbon",
  "CoreFoundation",
  "CoreGraphics",
  "CoreServices",
  "CoreVideo",
  "Foundation",
  "IOKit",
  "LocalAuthentication",
  "Metal",
  "Quartz",
  "QuartzCore",
  "Security",
  "ServiceManagement",
  "SwiftUI",
  "UserNotifications",
];

/**
 * Locate the host archive for `target`: the one staged in the installed `@quickgui/native`
 * package, or a source checkout's own release build.
 */
export function resolveHostLibrary(target: QuickGuiTarget, projectRoot: string): HostLibrary {
  const nativeDir = resolvePackageDir("@quickgui/native", projectRoot);
  const staged = join(nativeDir, "lib", target);
  const stagedArchive = join(staged, "libquickgui_host.a");
  const recipePath = join(staged, "link.json");
  if (existsSync(stagedArchive) && existsSync(recipePath)) {
    const recipe = JSON.parse(readFileSync(recipePath, "utf8")) as LinkRecipe;
    return {
      archive: stagedArchive,
      entry: recipe.entry,
      frameworks: recipe.frameworks,
      libraries: recipe.libraries,
      searchPaths: recipe.searchPaths,
    };
  }
  const checkoutArchive = resolve(nativeDir, "..", "..", "target", "release", "libquickgui_host.a");
  const info = targetInfo(target);
  if (existsSync(checkoutArchive) && info.platform === "darwin") {
    return {
      archive: checkoutArchive,
      entry: "_quickgui_main",
      frameworks: macFrameworks,
      libraries: ["c++", "iconv", "objc", "swift_Concurrency"],
      searchPaths: ["/usr/lib/swift"],
    };
  }
  throw new CliError(
    `The installed @quickgui/native package does not contain the host library for ${target}. ` +
      "Run `bun run build:native` in a QuickGUI checkout or install a native package that supports this target.",
  );
}

/** One binding in a scriptc FFI manifest (format 5). */
export interface FfiFunction {
  name: string;
  symbol: string;
  params: unknown[];
  returns: string;
}

export interface NativeCompileOptions {
  config: ResolvedQuickGuiConfig;
  mode: "development" | "production";
  target: QuickGuiTarget;
  /** Where the executable is written. */
  executablePath: string;
  /** Font file names relative to the application's resource directory. */
  fonts: string[];
  /** Static libraries linked in addition to the host, such as native modules. */
  extraLibraries?: string[];
  /** FFI bindings declared by those libraries. */
  extraFunctions?: FfiFunction[];
}

interface FfiManifest {
  ffi_format: number;
  functions: FfiFunction[];
  libraries: string[];
  system_libraries: string[];
}

/** The scriptc compiler runs under Node.js: its TypeScript frontend needs Node's process internals. */
function resolveNode(): string {
  const node = Bun.which("node");
  if (!node) {
    throw new CliError("Native compilation needs Node.js on PATH: the scriptc compiler runs under Node");
  }
  const version = Bun.spawnSync([node, "--version"], { stdout: "pipe", stderr: "pipe" });
  const major = Number(version.stdout.toString().trim().replace(/^v/, "").split(".")[0]);
  if (version.exitCode !== 0 || !Number.isFinite(major) || major < 24) {
    throw new CliError("Native compilation needs Node.js 24 or later on PATH for scriptc");
  }
  return node;
}

/** Locate the scriptc CLI installed next to this package. */
function resolveScriptc(projectRoot: string): string {
  for (const from of [projectRoot, import.meta.dir]) {
    try {
      const packageJson = Bun.resolveSync("scriptc/package.json", from);
      const bin = join(dirname(packageJson), "dist", "bootstrap.js");
      if (existsSync(bin)) return bin;
    } catch {
      // Try the next location.
    }
  }
  throw new CliError("Could not resolve the scriptc compiler; install the `scriptc` package");
}

/** Compile the project into a native executable at `executablePath`. */
export async function compileNativeApplication(options: NativeCompileOptions): Promise<void> {
  const { config, target } = options;
  const info = targetInfo(target);
  if (info.platform !== "darwin") {
    throw new CliError(`Native builds currently target macOS; ${target} is not supported yet`);
  }
  if (process.platform !== "darwin" || process.arch !== info.architecture) {
    throw new CliError(`scriptc builds for the current machine; build ${target} on a matching macOS ${info.architecture} host`);
  }
  const minimumSystemVersion = config.macos.minimumSystemVersion;
  if (!/^\d+\.\d+(?:\.\d+)?$/.test(minimumSystemVersion) || Number(minimumSystemVersion.split(".")[0]) < 14) {
    throw new CliError("scriptc applications require macOS 14.0 or later; set macos.minimumSystemVersion to 14.0 or higher");
  }
  const host = resolveHostLibrary(target, config.projectRoot);
  const buildDir = resolve(config.projectRoot, ".quickgui", "native", options.mode);
  const lowered = await lowerProject({
    projectRoot: config.projectRoot,
    entry: config.entry,
    outDir: buildDir,
    name: config.name,
    version: config.version,
    identifier: options.mode === "development" ? `${config.identifier}.dev` : config.identifier,
    fonts: options.fonts,
    typeCheck: config.native.typeCheck,
  });

  const manifestSource = join(lowered.nativeDir, "ffi.json");
  const manifest = JSON.parse(readFileSync(manifestSource, "utf8")) as FfiManifest;
  // Module libraries precede the host: they call back into it to report asynchronous results.
  manifest.libraries = [...(options.extraLibraries ?? []), host.archive];
  manifest.system_libraries = host.libraries;
  if (options.extraFunctions !== undefined) manifest.functions = [...manifest.functions, ...options.extraFunctions];
  const manifestPath = join(buildDir, "ffi.json");
  writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);

  const linkerPath = join(buildDir, "link.sh");
  const linkerLogPath = join(buildDir, "link.log");
  writeFileSync(linkerLogPath, "");
  const linkerFlags = [
    `-mmacosx-version-min=${minimumSystemVersion}`,
    `-Wl,-e,${host.entry}`,
    "-Wl,-dead_strip",
    ...host.searchPaths.map((path) => `-L${path}`),
    ...host.frameworks.flatMap((framework) => ["-framework", framework]),
  ];
  // scriptc may only retain clang's final summary. Preserve the actual linker diagnostics too.
  writeFileSync(linkerPath, `#!/bin/sh\nclang "$@" ${linkerFlags.map(shellQuote).join(" ")} >${shellQuote(linkerLogPath)} 2>&1\nstatus=$?\ncat ${shellQuote(linkerLogPath)} >&2\nexit "$status"\n`);
  chmodSync(linkerPath, 0o755);

  const scriptc = resolveScriptc(config.projectRoot);
  const arguments_ = [
    resolveNode(),
    scriptc,
    "build",
    lowered.entryPath,
    "--ffi",
    manifestPath,
    "-o",
    options.executablePath,
    // Development builds trade optimization for scriptc's cached object shards: a rebuild after an
    // edit relinks in well under a second instead of recompiling the whole program.
    ...(options.mode === "development" ? ["--optimization", "dev"] : []),
    ...(config.native.dynamic ? ["--dynamic"] : []),
  ];
  const child = Bun.spawn(arguments_, {
    cwd: buildDir,
    stdin: "ignore",
    stdout: "pipe",
    stderr: "pipe",
    env: { ...process.env, SCRIPTC_LINKER: linkerPath, NODE_ENV: options.mode },
  });
  const [status, stdout, stderr] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);
  if (status !== 0) {
    const detail = (stderr.trim() || stdout.trim()).replace(/\x1b\[[0-9;]*m/g, "");
    const linkerDetail = readFileSync(linkerLogPath, "utf8").trim();
    throw new CliError(`Native compilation failed\n${detail}${linkerDetail && !detail.includes(linkerDetail) ? `\n${linkerDetail}` : ""}`);
  }
  if (!existsSync(options.executablePath)) {
    throw new CliError(`scriptc reported success but ${options.executablePath} is missing`);
  }
}

function shellQuote(value: string): string {
  return /^[A-Za-z0-9_./=,+-]+$/.test(value) ? value : `'${value.replaceAll("'", "'\\''")}'`;
}
