/**
 * Native modules: Zig under `modules/<name>/main.zig`, compiled into static libraries the
 * application links, and exposed to it as typed TypeScript modules.
 *
 * `buildNativeModules` is the whole pipeline. For every module directory it hashes the Zig
 * sources, compiles them with `zig build-lib` against the runtime shipped in `@quickgui/native`
 * when the hash changed, reads the manifest by linking a tiny probe program against the library,
 * and writes `modules/<name>/index.ts`: typed wrappers around the module's two C entry points.
 * The library lives under `.quickgui/modules/<name>/<target>/`; when the application is compiled,
 * the CLI links it and binds the entry points through the scriptc FFI manifest.
 *
 * The pure pieces — argument lists, the generator, and the Zig signature scanner — are exported
 * so tests can cover them without a compiler.
 */

import { createHash } from "node:crypto";
import {
  existsSync,
  chmodSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  realpathSync,
  renameSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";

import { NATIVE_MODULE_ABI, type NativeWire } from "@quickgui/native/modules";

import type { ResolvedQuickGuiConfig, ZigOptimizeMode } from "./config.ts";
import { CliError } from "./error.ts";
import type { FfiFunction } from "./native-build.ts";
import { hostTarget, targetInfo, type QuickGuiTarget } from "./targets.ts";

/** The file that makes a directory under `modules/` a native module. */
export const NATIVE_MODULE_ENTRY = "main.zig";
/** Oldest Zig the runtime in `@quickgui/native/zig` targets. */
export const MINIMUM_ZIG_VERSION = { major: 0, minor: 16 } as const;
/** Most native modules one project may declare. */
export const MAX_NATIVE_MODULES = 64;
/** Most exported functions one native module may declare. */
export const MAX_NATIVE_MODULE_FUNCTIONS = 1_024;

// --- manifest ------------------------------------------------------------------------------------

/** Description of one Zig type, as emitted by the module runtime's manifest. */
export type NativeTypeDescriptor =
  | { kind: "void" }
  | { kind: "boolean" }
  | { kind: "number" }
  | { kind: "string" }
  | { kind: "bytes" }
  | { kind: "any" }
  | { kind: "optional"; inner: NativeTypeDescriptor }
  | { kind: "array"; element: NativeTypeDescriptor }
  | { kind: "tuple"; elements: NativeTypeDescriptor[] }
  | { kind: "struct"; name: string; fields: NativeStructField[] }
  | { kind: "enum"; name: string; values: string[] }
  | { kind: "union"; name: string; variants: NativeUnionVariant[] }
  | { kind: "ref"; name: string };

export interface NativeStructField {
  name: string;
  type: NativeTypeDescriptor;
  /** Whether the Zig field has a default value, so the application may omit it. */
  hasDefault: boolean;
}

export interface NativeUnionVariant {
  name: string;
  type: NativeTypeDescriptor;
}

export interface NativeValueSpec {
  wire: NativeWire;
  type: NativeTypeDescriptor;
}

/** A parameter the runtime supplies itself instead of reading from the application. */
export interface NativeInjectedParameter {
  injected: "allocator";
}

export type NativeParameterSpec = NativeValueSpec | NativeInjectedParameter;

export interface NativeFunctionSpec {
  name: string;
  params: NativeParameterSpec[];
  result: NativeValueSpec;
}

/** The manifest a compiled module reports from its `_manifest` entry point. */
export interface NativeModuleManifest {
  abi: number;
  /** Version of the Zig compiler that built the module. */
  zig: string;
  functions: NativeFunctionSpec[];
}

const wireNames: readonly string[] = ["void", "boolean", "number", "string", "bytes", "json"];

/** Parse and validate the JSON a module reports from its manifest entry point. */
export function parseNativeModuleManifest(text: string): NativeModuleManifest {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch (error) {
    throw new Error("The native module manifest is not valid JSON", { cause: error });
  }
  if (!isRecord(parsed) || typeof parsed.abi !== "number" || !Array.isArray(parsed.functions)) {
    throw new Error("The native module manifest does not have the expected shape");
  }
  if (parsed.abi !== NATIVE_MODULE_ABI) {
    throw new Error(
      `The native module was built for ABI ${parsed.abi}, but this @quickgui/native expects ABI ${NATIVE_MODULE_ABI}`,
    );
  }
  const functions = parsed.functions.map((entry, index): NativeFunctionSpec => {
    if (
      !isRecord(entry) ||
      typeof entry.name !== "string" ||
      !Array.isArray(entry.params) ||
      !isValueSpec(entry.result)
    ) {
      throw new Error(`Native module function ${index} has an invalid manifest entry`);
    }
    const params = entry.params.map((parameter): NativeParameterSpec => {
      if (isRecord(parameter) && parameter.injected === "allocator") return { injected: "allocator" };
      if (isValueSpec(parameter)) return parameter;
      throw new Error(`Native module function ${entry.name} has an invalid parameter entry`);
    });
    return { name: entry.name, params, result: entry.result };
  });
  return {
    abi: parsed.abi,
    zig: typeof parsed.zig === "string" ? parsed.zig : "",
    functions,
  };
}

function isValueSpec(value: unknown): value is NativeValueSpec {
  return (
    isRecord(value) &&
    typeof value.wire === "string" &&
    wireNames.includes(value.wire) &&
    isRecord(value.type) &&
    typeof value.type.kind === "string"
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

// --- module identity -----------------------------------------------------------------------------

export interface NativeModuleSource {
  /** Directory name, which is also the addon name and the generated module's identity. */
  name: string;
  directory: string;
  /** Absolute path of `main.zig`. */
  entry: string;
}

export interface ZigToolchain {
  path: string;
  version: string;
}

export interface BuildNativeModulesOptions {
  target: QuickGuiTarget;
  mode: "development" | "production";
  /** Receives one progress line per compiled module. */
  log?: (line: string) => void;
}

export interface BuiltNativeModule {
  name: string;
  /** The static library the application links, under `.quickgui/modules/<name>/<target>/`. */
  archivePath: string;
  /** The generated `modules/<name>/index.ts`. */
  indexPath: string;
  manifest: NativeModuleManifest;
  /** Whether Zig ran, as opposed to the previous library being reused. */
  rebuilt: boolean;
  /** The prefix of the module's C entry points, `quickgui_module_<name>`. */
  symbolPrefix: string;
  /** scriptc FFI manifest entries that bind the generated wrappers to the library. */
  ffiFunctions: FfiFunction[];
}

/** The C symbol prefix of a module's entry points. */
export function moduleSymbolPrefix(name: string): string {
  return `quickgui_module_${name.replaceAll("-", "_")}`;
}

/** Root the CLI compiles: the module's public functions become the library's entry points. */
export function nativeModuleRootSource(symbolPrefix: string): string {
  return `// Generated by @quickgui/cli. Do not edit.
comptime {
    @import("quickgui").exportModule(@import("module"), "${symbolPrefix}");
}
`;
}

/**
 * The scriptc FFI entries for one module: the synchronous call hands its result to a
 * call-scoped callback, the asynchronous one reports through the host as a `module-result` event.
 */
export function nativeModuleFfiFunctions(symbolPrefix: string): FfiFunction[] {
  return [
    {
      name: `${symbolPrefix}_call`,
      symbol: `${symbolPrefix}_call`,
      params: [
        "u32",
        "bytes",
        { callback: { id: "reply", params: ["bytes", { context: "reply" }], returns: "void", lifetime: "call" } },
        { context: "reply" },
      ],
      returns: "void",
    },
    {
      name: `${symbolPrefix}_call_async`,
      symbol: `${symbolPrefix}_call_async`,
      params: ["u32", "u32", "bytes"],
      returns: "void",
    },
  ];
}

/** A C program that prints the module's manifest, linked against the module library. */
export function manifestProbeSource(symbolPrefix: string): string {
  return `#include <stddef.h>
#include <stdio.h>
const char *${symbolPrefix}_manifest(void);
void quickgui_module_complete(unsigned int request, const unsigned char *data, size_t length) {
    (void)request;
    (void)data;
    (void)length;
}
int main(void) {
    fputs(${symbolPrefix}_manifest(), stdout);
    return 0;
}
`;
}

// --- discovery -----------------------------------------------------------------------------------

/** Every `<directory>/<name>/main.zig`, sorted by name. */
export function discoverNativeModules(directory: string): NativeModuleSource[] {
  if (!existsSync(directory) || !statSync(directory).isDirectory()) return [];
  const modules: NativeModuleSource[] = [];
  const entries = readdirSync(directory, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort();
  for (const name of entries) {
    const entry = join(directory, name, NATIVE_MODULE_ENTRY);
    if (!existsSync(entry) || !statSync(entry).isFile()) continue;
    if (!/^[A-Za-z][A-Za-z0-9_-]*$/.test(name)) {
      throw new CliError(
        `Native module directory name \`${name}\` must start with a letter and contain only letters, digits, hyphens, and underscores`,
      );
    }
    modules.push({ name, directory: join(directory, name), entry });
  }
  if (modules.length > MAX_NATIVE_MODULES) {
    throw new CliError(`A project may declare at most ${MAX_NATIVE_MODULES} native modules`);
  }
  return modules;
}

// --- toolchain -----------------------------------------------------------------------------------

export function parseZigVersion(
  text: string,
): { major: number; minor: number; patch: number } | undefined {
  const match = /^(\d+)\.(\d+)\.(\d+)/.exec(text.trim());
  if (!match) return undefined;
  return { major: Number(match[1]), minor: Number(match[2]), patch: Number(match[3]) };
}

export function zigVersionSupported(version: string): boolean {
  const parsed = parseZigVersion(version);
  if (!parsed) return false;
  if (parsed.major !== MINIMUM_ZIG_VERSION.major) return parsed.major > MINIMUM_ZIG_VERSION.major;
  return parsed.minor >= MINIMUM_ZIG_VERSION.minor;
}

/** Locate a usable Zig: `QUICKGUI_ZIG`, then `zig` on PATH. */
export function findZig(env: Record<string, string | undefined> = process.env): ZigToolchain {
  const candidates: string[] = [];
  if (env.QUICKGUI_ZIG) candidates.push(env.QUICKGUI_ZIG);
  const onPath = Bun.which("zig", env.PATH ? { PATH: env.PATH } : undefined);
  if (onPath) candidates.push(onPath);
  const seen: string[] = [];
  for (const candidate of candidates) {
    if (!existsSync(candidate)) continue;
    const result = Bun.spawnSync([candidate, "version"], { stdout: "pipe", stderr: "pipe" });
    if (result.exitCode !== 0) continue;
    const version = result.stdout.toString().trim();
    if (zigVersionSupported(version)) return { path: candidate, version };
    seen.push(`${candidate} (${version})`);
  }
  const minimum = `${MINIMUM_ZIG_VERSION.major}.${MINIMUM_ZIG_VERSION.minor}`;
  throw new CliError(
    `Native modules need Zig ${minimum} or newer on PATH, or QUICKGUI_ZIG pointing at one. ` +
      (seen.length > 0 ? `Found ${seen.join(", ")}. ` : "") +
      "Install it from https://ziglang.org/download/ (for example `mise use -g zig@0.16.0`).",
  );
}

export function zigTarget(target: QuickGuiTarget): string {
  const info = targetInfo(target);
  if (info.platform === "windows") {
    throw new CliError(
      `Native modules are not supported for ${target} yet: native applications target macOS first`,
    );
  }
  const architecture = info.architecture === "arm64" ? "aarch64" : "x86_64";
  return info.platform === "darwin" ? `${architecture}-macos` : `${architecture}-linux-gnu`;
}

export interface ZigBuildInput {
  zig: string;
  name: string;
  optimize: ZigOptimizeMode;
  target: QuickGuiTarget;
  /** The generated root that exports the module. */
  root: string;
  /** The module's `main.zig`. */
  entry: string;
  /** `quickgui.zig` from `@quickgui/native/zig`. */
  runtime: string;
  /** Where the static library is written. */
  output: string;
  cacheDirectory: string;
}

/**
 * The `zig build-lib` invocation for one module.
 *
 * The library links libc because the runtime runs asynchronous calls on threads; the host
 * symbol it reports results through stays undefined until the application is linked.
 */
export function zigBuildArguments(input: ZigBuildInput): string[] {
  return [
    input.zig,
    "build-lib",
    "-static",
    `-O${input.optimize}`,
    "-target",
    zigTarget(input.target),
    "-lc",
    "--name",
    input.name,
    `-femit-bin=${input.output}`,
    "--cache-dir",
    input.cacheDirectory,
    "--dep",
    "quickgui",
    "--dep",
    "module",
    `-Mroot=${input.root}`,
    "--dep",
    "quickgui",
    `-Mmodule=${input.entry}`,
    `-Mquickgui=${input.runtime}`,
  ];
}

/** The `zig cc` invocation that links the manifest probe against a module library. */
export function probeBuildArguments(zig: string, source: string, archive: string, output: string): string[] {
  return [zig, "cc", "-o", output, source, archive];
}

// --- build ---------------------------------------------------------------------------------------

interface BuildContext {
  zig: ZigToolchain;
  optimize: ZigOptimizeMode;
  runtimeDirectory: string;
  cacheDirectory: string;
  projectRoot: string;
}

interface ArchiveBuild {
  archivePath: string;
  statePath: string;
  hash: string;
  /** The manifest recorded by an earlier build of the same inputs, when the library is current. */
  manifest: NativeModuleManifest | undefined;
}

/** Compile every native module of a project and refresh its generated `index.ts`. */
export async function buildNativeModules(
  config: ResolvedQuickGuiConfig,
  options: BuildNativeModulesOptions,
): Promise<BuiltNativeModule[]> {
  const modules = discoverNativeModules(config.modules.directory);
  if (modules.length === 0) return [];
  zigTarget(options.target);
  const context: BuildContext = {
    zig: findZig(),
    optimize: config.modules.optimize ?? (options.mode === "production" ? "ReleaseFast" : "ReleaseSafe"),
    runtimeDirectory: nativeRuntimeDirectory(),
    cacheDirectory: resolve(config.projectRoot, ".quickgui", "zig-cache"),
    projectRoot: config.projectRoot,
  };
  const host = hostTarget();
  const prefixes = new Map<string, string>();
  const results: BuiltNativeModule[] = [];
  for (const module of modules) {
    const symbolPrefix = moduleSymbolPrefix(module.name);
    const clash = prefixes.get(symbolPrefix);
    if (clash !== undefined) {
      throw new CliError(
        `Native modules ${clash} and ${module.name} would export the same C symbols; rename one of them`,
      );
    }
    prefixes.set(symbolPrefix, module.name);
    const started = performance.now();
    const archive = await ensureArchive(module, symbolPrefix, options.target, context);
    let manifest = archive.manifest;
    let rebuilt = false;
    if (manifest === undefined) {
      // The manifest comes from running a probe, so a cross-compiled module also gets a host build.
      const probe = options.target === host ? archive : await ensureArchive(module, symbolPrefix, host, context);
      manifest =
        probe.manifest ??
        (await readArchiveManifest(context, probe.archivePath, symbolPrefix, dirname(probe.archivePath), module.name));
      if (manifest.functions.length > MAX_NATIVE_MODULE_FUNCTIONS) {
        throw new CliError(
          `Native module ${module.name} exports ${manifest.functions.length} functions; the limit is ${MAX_NATIVE_MODULE_FUNCTIONS}`,
        );
      }
      writeFileSync(archive.statePath, `${JSON.stringify({ hash: archive.hash, manifest }, null, 2)}\n`);
      if (probe !== archive && probe.manifest === undefined) {
        writeFileSync(probe.statePath, `${JSON.stringify({ hash: probe.hash, manifest }, null, 2)}\n`);
      }
      rebuilt = true;
      options.log?.(
        `Built native module ${module.name} with Zig ${context.zig.version} (${context.optimize}) in ${Math.round(performance.now() - started)} ms`,
      );
    }
    const indexPath = resolve(module.directory, "index.ts");
    const source = generateNativeModuleSource({
      name: module.name,
      manifest,
      entryDisplayPath: relative(config.projectRoot, module.entry).split(sep).join("/"),
      zigSource: readFileSync(module.entry, "utf8"),
    });
    if (!existsSync(indexPath) || readFileSync(indexPath, "utf8") !== source) {
      writeFileSync(indexPath, source);
    }
    results.push({
      name: module.name,
      archivePath: archive.archivePath,
      indexPath,
      manifest,
      rebuilt,
      symbolPrefix,
      ffiFunctions: nativeModuleFfiFunctions(symbolPrefix),
    });
  }
  return results;
}

/** Compile the module's library for `target` unless the recorded build already matches its inputs. */
async function ensureArchive(
  module: NativeModuleSource,
  symbolPrefix: string,
  target: QuickGuiTarget,
  context: BuildContext,
): Promise<ArchiveBuild> {
  const outputDirectory = resolve(context.projectRoot, ".quickgui", "modules", module.name, target);
  mkdirSync(outputDirectory, { recursive: true });
  const archivePath = resolve(outputDirectory, `lib${module.name}.a`);
  const statePath = resolve(outputDirectory, "build.json");
  const hash = nativeModuleInputsHash(module, {
    zigVersion: context.zig.version,
    target,
    optimize: context.optimize,
    runtimeDirectory: context.runtimeDirectory,
    symbolPrefix,
  });
  const manifest = readBuildState(statePath, hash, archivePath);
  if (manifest !== undefined) return { archivePath, statePath, hash, manifest };
  const rootPath = resolve(outputDirectory, "root.zig");
  writeFileSync(rootPath, nativeModuleRootSource(symbolPrefix));
  rmSync(archivePath, { force: true });
  await runZig(
    zigBuildArguments({
      zig: context.zig.path,
      name: module.name,
      optimize: context.optimize,
      target,
      root: rootPath,
      entry: module.entry,
      runtime: resolve(context.runtimeDirectory, "quickgui.zig"),
      output: archivePath,
      cacheDirectory: context.cacheDirectory,
    }),
    context.projectRoot,
    module.name,
  );
  if (process.platform === "darwin" && targetInfo(target).platform === "darwin") {
    // Zig's archive writer can leave Mach-O members only two-byte aligned. Apple's linker
    // requires eight-byte alignment; libtool rewrites the archive and its symbol index.
    const objectsDirectory = mkdtempSync(join(outputDirectory, "objects-"));
    try {
      // libtool skips misaligned archive members, so give it extracted object files.
      await runZig([context.zig.path, "ar", "x", archivePath], objectsDirectory, module.name);
      const objects = readdirSync(objectsDirectory).filter((name) => name.endsWith(".o")).map((name) => join(objectsDirectory, name));
      if (objects.length === 0) throw new CliError(`Native module ${module.name} produced an empty archive`);
      // Zig's deterministic archive headers use mode 000, which extraction preserves.
      for (const object of objects) chmodSync(object, 0o644);
      const alignedPath = resolve(outputDirectory, `lib${module.name}.aligned.a`);
      await runZig(["xcrun", "libtool", "-static", "-o", alignedPath, ...objects], context.projectRoot, module.name);
      renameSync(alignedPath, archivePath);
    } finally {
      rmSync(objectsDirectory, { recursive: true, force: true });
    }
  }
  return { archivePath, statePath, hash, manifest: undefined };
}

function nativeRuntimeDirectory(): string {
  let entry: string;
  try {
    entry = Bun.resolveSync("@quickgui/native", import.meta.dir);
  } catch (error) {
    throw new CliError("Could not resolve @quickgui/native, which ships the native module runtime", {
      cause: error,
    });
  }
  const directory = resolve(dirname(realpathSync(entry)), "zig");
  if (!existsSync(resolve(directory, "quickgui.zig"))) {
    throw new CliError(`The installed @quickgui/native package has no zig/quickgui.zig at ${directory}`);
  }
  return directory;
}

/** Every `.zig` file under a directory, relative to it, sorted. */
export function zigSourceFiles(directory: string): string[] {
  const files: string[] = [];
  const walk = (current: string): void => {
    for (const entry of readdirSync(current, { withFileTypes: true })) {
      if (entry.name === ".zig-cache" || entry.name === "zig-out" || entry.name === "node_modules") {
        continue;
      }
      const path = join(current, entry.name);
      if (entry.isDirectory()) walk(path);
      else if (entry.isFile() && entry.name.endsWith(".zig")) files.push(relative(directory, path));
    }
  };
  walk(directory);
  return files.sort();
}

function nativeModuleInputsHash(
  module: NativeModuleSource,
  inputs: { zigVersion: string; target: string; optimize: string; runtimeDirectory: string; symbolPrefix: string },
): string {
  const hash = createHash("sha256");
  hash.update(`quickgui-native-module:${NATIVE_MODULE_ABI}\0`);
  hash.update(`mach-o-alignment-v2\0${inputs.zigVersion}\0${inputs.target}\0${inputs.optimize}\0`);
  hash.update(nativeModuleRootSource(inputs.symbolPrefix));
  for (const file of zigSourceFiles(inputs.runtimeDirectory)) {
    hash.update(`runtime:${file}\0`);
    hash.update(readFileSync(join(inputs.runtimeDirectory, file)));
    hash.update("\0");
  }
  for (const file of zigSourceFiles(module.directory)) {
    hash.update(`module:${file}\0`);
    hash.update(readFileSync(join(module.directory, file)));
    hash.update("\0");
  }
  return hash.digest("hex");
}

function readBuildState(
  statePath: string,
  hash: string,
  addonPath: string,
): NativeModuleManifest | undefined {
  if (!existsSync(statePath) || !existsSync(addonPath)) return undefined;
  try {
    const state = JSON.parse(readFileSync(statePath, "utf8")) as { hash?: unknown; manifest?: unknown };
    if (state.hash !== hash || state.manifest === undefined) return undefined;
    return parseNativeModuleManifest(JSON.stringify(state.manifest));
  } catch {
    return undefined;
  }
}

async function runZig(command: string[], cwd: string, moduleName: string): Promise<void> {
  const child = Bun.spawn(command, { cwd, stdin: "ignore", stdout: "pipe", stderr: "pipe" });
  const [status, stdout, stderr] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);
  if (status !== 0) {
    const detail = (stderr.trim() || stdout.trim()).replaceAll(cwd + sep, "");
    throw new CliError(
      `Zig could not compile native module ${moduleName}${detail ? `\n${detail}` : ""}`,
    );
  }
}

/**
 * Link a tiny C program against the module library, run it, and return the manifest it prints.
 *
 * The probe stubs the host symbol the library reports asynchronous results through, so it links
 * without the QuickGUI host; the manifest itself is computed at Zig compile time.
 */
async function readArchiveManifest(
  context: BuildContext,
  archivePath: string,
  symbolPrefix: string,
  outputDirectory: string,
  moduleName: string,
): Promise<NativeModuleManifest> {
  const probeSource = resolve(outputDirectory, "manifest-probe.c");
  const probeBinary = resolve(outputDirectory, "manifest-probe");
  writeFileSync(probeSource, manifestProbeSource(symbolPrefix));
  const link = Bun.spawn(probeBuildArguments(context.zig.path, probeSource, archivePath, probeBinary), {
    cwd: outputDirectory,
    stdin: "ignore",
    stdout: "pipe",
    stderr: "pipe",
    env: { ...process.env, ZIG_LOCAL_CACHE_DIR: context.cacheDirectory },
  });
  const [linkStatus, linkStdout, linkStderr] = await Promise.all([
    link.exited,
    new Response(link.stdout).text(),
    new Response(link.stderr).text(),
  ]);
  if (linkStatus !== 0) {
    const detail = (linkStderr.trim() || linkStdout.trim()).replaceAll(context.projectRoot + sep, "");
    throw new CliError(
      `Could not link the manifest probe of native module ${moduleName}${detail ? `\n${detail}` : ""}`,
    );
  }
  const run = Bun.spawn([probeBinary], { cwd: outputDirectory, stdin: "ignore", stdout: "pipe", stderr: "pipe" });
  const [status, stdout, stderr] = await Promise.all([
    run.exited,
    new Response(run.stdout).text(),
    new Response(run.stderr).text(),
  ]);
  if (status !== 0) {
    throw new CliError(
      `The manifest probe of native module ${moduleName} failed${stderr.trim() ? `\n${stderr.trim()}` : ""}`,
    );
  }
  try {
    return parseNativeModuleManifest(stdout);
  } catch (error) {
    throw new CliError(`Native module ${moduleName} reported an invalid manifest`, { cause: error });
  }
}

// --- Zig source scanning -------------------------------------------------------------------------

export interface ZigFunctionSignature {
  name: string;
  /** Parameter names in declaration order; `_` becomes an empty string. */
  params: string[];
  /** `///` comment lines directly above the function, without the marker. */
  doc: string[];
  /** The declaration from `pub fn` up to the body, on one line. */
  text: string;
}

/**
 * Recover what `@typeInfo` does not carry: parameter names and doc comments of every `pub fn`.
 *
 * This is a lexical scan, not a parser; it handles the shapes real modules use (nested parentheses
 * in types, multi-line parameter lists, string literals) and leaves anything else nameless.
 */
export function extractZigFunctionSignatures(source: string): Map<string, ZigFunctionSignature> {
  const signatures = new Map<string, ZigFunctionSignature>();
  const lines = source.split("\n");
  let doc: string[] = [];
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index]!;
    const docMatch = /^\s*\/\/\/(.*)$/.exec(line);
    if (docMatch) {
      doc.push(docMatch[1]!.replace(/^ /, ""));
      continue;
    }
    const match = /^\s*pub\s+(?:inline\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(/.exec(line);
    if (!match) {
      if (line.trim().length > 0) doc = [];
      continue;
    }
    const name = match[1]!;
    // Collect text from the opening parenthesis until the body starts.
    let text = line.slice(line.indexOf("("));
    let closed = -1;
    let cursor = index;
    for (;;) {
      let depth = 0;
      for (let position = 0; position < text.length; position += 1) {
        const character = text[position];
        if (character === "(") depth += 1;
        else if (character === ")") {
          depth -= 1;
          if (depth === 0) {
            closed = position;
            break;
          }
        }
      }
      if (closed >= 0) break;
      cursor += 1;
      if (cursor >= lines.length) break;
      text += `\n${lines[cursor]}`;
    }
    if (closed < 0) {
      doc = [];
      continue;
    }
    let declaration = text.slice(0, closed + 1);
    let rest = text.slice(closed + 1);
    let bodyAt = findBodyBrace(rest);
    while (bodyAt < 0 && cursor + 1 < lines.length) {
      cursor += 1;
      rest += `\n${lines[cursor]}`;
      bodyAt = findBodyBrace(rest);
    }
    declaration += bodyAt >= 0 ? rest.slice(0, bodyAt) : rest;
    const params = splitTopLevel(declaration.slice(1, closed)).map((parameter) => {
      const cleaned = parameter.replace(/^\s*(?:comptime|noalias)\s+/, "").trim();
      const colon = cleaned.indexOf(":");
      const identifier = (colon >= 0 ? cleaned.slice(0, colon) : cleaned).trim();
      if (identifier === "_" || !/^[A-Za-z_][A-Za-z0-9_]*$/.test(identifier)) return "";
      return identifier;
    });
    signatures.set(name, {
      name,
      params,
      doc,
      text: `pub fn ${name}${declaration.replace(/\s+/g, " ").trim()}`,
    });
    doc = [];
    index = cursor;
  }
  return signatures;
}

/**
 * Index of the `{` that opens the function body in the text after the parameter list, or -1 when
 * the text ends first. A `{` that follows `struct`, `union`, `enum`, `opaque`, or `error` (possibly
 * with a parenthesized tag or backing type) belongs to an anonymous type in the return type and
 * is skipped together with its contents.
 */
function findBodyBrace(text: string): number {
  const typeKeywords = new Set(["struct", "union", "enum", "opaque", "error"]);
  let position = 0;
  while (position < text.length) {
    const brace = text.indexOf("{", position);
    if (brace < 0) return -1;
    let before = brace;
    while (before > 0 && /\s/.test(text[before - 1]!)) before -= 1;
    let keyword: string | undefined;
    if (text[before - 1] === ")") {
      let depth = 0;
      let open = before - 1;
      for (; open >= 0; open -= 1) {
        if (text[open] === ")") depth += 1;
        else if (text[open] === "(") {
          depth -= 1;
          if (depth === 0) break;
        }
      }
      const identifier = /([A-Za-z_][A-Za-z0-9_]*)\s*$/.exec(text.slice(0, Math.max(open, 0)));
      keyword = identifier?.[1];
    } else {
      const identifier = /([A-Za-z_][A-Za-z0-9_]*)$/.exec(text.slice(0, before));
      keyword = identifier?.[1];
    }
    if (keyword === undefined || !typeKeywords.has(keyword)) return brace;
    // Skip the anonymous type's braces; an unclosed one needs more lines.
    let depth = 0;
    let index = brace;
    for (; index < text.length; index += 1) {
      if (text[index] === "{") depth += 1;
      else if (text[index] === "}") {
        depth -= 1;
        if (depth === 0) break;
      }
    }
    if (index >= text.length) return -1;
    position = index + 1;
  }
  return -1;
}

function splitTopLevel(text: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let current = "";
  for (const character of text) {
    if (character === "(" || character === "[" || character === "{") depth += 1;
    else if (character === ")" || character === "]" || character === "}") depth -= 1;
    if (character === "," && depth === 0) {
      parts.push(current);
      current = "";
      continue;
    }
    current += character;
  }
  if (current.trim().length > 0) parts.push(current);
  return parts.filter((part) => part.trim().length > 0);
}

// --- TypeScript generation -----------------------------------------------------------------------

export interface GenerateNativeModuleSourceInput {
  name: string;
  manifest: NativeModuleManifest;
  /** `main.zig` relative to the project root, for the header comment. */
  entryDisplayPath: string;
  /** Contents of `main.zig`, for parameter names and doc comments. */
  zigSource?: string;
}

const reservedIdentifiers = new Set([
  "arguments", "await", "break", "case", "catch", "class", "const", "continue", "debugger",
  "default", "delete", "do", "else", "enum", "eval", "export", "extends", "false", "finally",
  "for", "function", "if", "implements", "import", "in", "instanceof", "interface", "let", "new",
  "null", "package", "private", "protected", "public", "return", "static", "super", "switch",
  "this", "throw", "true", "try", "typeof", "undefined", "var", "void", "while", "with", "yield",
  // Identifiers the generated file uses itself.
  "args", "call", "callAsync", "request", "result", "bytes", "JSON", "Promise", "Uint8Array",
  "NativeArguments", "allocateNativeModuleRequest", "awaitNativeModuleResult", "decodeNativeVoid",
  "decodeNativeBoolean", "decodeNativeNumber", "decodeNativeString", "decodeNativeBytes",
  "decodeNativeJson",
]);

const identifierPattern = /^[A-Za-z_$][A-Za-z0-9_$]*$/;

interface HoistedType {
  tsName: string;
  zigName: string;
  descriptor: NativeTypeDescriptor;
  /** Serialized descriptor, to tell two Zig types with one short name apart. */
  shape: string;
}

const decoderNames: Record<NativeWire, string> = {
  void: "decodeNativeVoid",
  boolean: "decodeNativeBoolean",
  number: "decodeNativeNumber",
  string: "decodeNativeString",
  bytes: "decodeNativeBytes",
  json: "decodeNativeJson",
};

/** Write the typed TypeScript module for one compiled native module. */
export function generateNativeModuleSource(input: GenerateNativeModuleSourceInput): string {
  const { name, manifest } = input;
  const symbolPrefix = moduleSymbolPrefix(name);
  const signatures = input.zigSource ? extractZigFunctionSignatures(input.zigSource) : new Map();
  const hoisted = new Map<string, HoistedType>();
  const hoistedByShape = new Map<string, HoistedType>();
  const declarationOrder: HoistedType[] = [];

  const hoist = (descriptor: NativeTypeDescriptor & { name: string }): HoistedType | undefined => {
    const shortName = hoistedName(descriptor.name);
    if (!shortName) return undefined;
    const shape = JSON.stringify(descriptor);
    const key = `${descriptor.name}\0${shape}`;
    const existing = hoistedByShape.get(key);
    if (existing) return existing;
    let tsName = shortName;
    if (hoisted.has(tsName)) {
      tsName = descriptor.name.replace(/[^A-Za-z0-9_$]+/g, "_");
      let counter = 2;
      while (hoisted.has(tsName)) tsName = `${shortName}${counter++}`;
    }
    const entry: HoistedType = { tsName, zigName: descriptor.name, descriptor, shape };
    hoisted.set(tsName, entry);
    hoistedByShape.set(key, entry);
    declarationOrder.push(entry);
    return entry;
  };

  const seenRefs = new Map<string, string>();
  const tsType = (descriptor: NativeTypeDescriptor): string => {
    switch (descriptor.kind) {
      case "void":
        return "Record<string, never>";
      case "boolean":
        return "boolean";
      case "number":
        return "number";
      case "string":
        return "string";
      case "bytes":
        return "Uint8Array";
      case "any":
        return "unknown";
      case "optional":
        return `${tsType(descriptor.inner)} | null`;
      case "array":
        return `${parenthesize(tsType(descriptor.element))}[]`;
      case "tuple":
        return `[${descriptor.elements.map(tsType).join(", ")}]`;
      case "ref":
        return seenRefs.get(descriptor.name) ?? "unknown";
      case "enum": {
        const entry = hoist(descriptor);
        return entry ? entry.tsName : enumLiteral(descriptor.values);
      }
      case "struct": {
        const entry = hoist(descriptor);
        if (entry) {
          seenRefs.set(descriptor.name, entry.tsName);
          return entry.tsName;
        }
        return structLiteral(descriptor.fields, tsType, " ");
      }
      case "union": {
        const entry = hoist(descriptor);
        if (entry) {
          seenRefs.set(descriptor.name, entry.tsName);
          return entry.tsName;
        }
        return unionLiteral(descriptor.variants, tsType);
      }
    }
  };

  // Resolve every function's types first so hoisted declarations are complete before emission.
  const functions = manifest.functions.map((spec) => {
    validateFunctionName(name, spec.name, manifest.functions);
    const signature = signatures.get(spec.name) as ZigFunctionSignature | undefined;
    const sourceNames = signature && signature.params.length === spec.params.length ? signature.params : undefined;
    const parameters: { name: string; type: string; wire: NativeWire }[] = [];
    let position = 0;
    spec.params.forEach((parameter, index) => {
      if ("injected" in parameter) return;
      position += 1;
      const candidate = sourceNames?.[index] || `arg${position}`;
      const parameterName =
        identifierPattern.test(candidate) && !reservedIdentifiers.has(candidate) && candidate !== "_"
          ? candidate
          : `${candidate.replace(/[^A-Za-z0-9_$]+/g, "_") || "arg"}_`;
      parameters.push({
        name: parameterName,
        type: parameterType(parameter.wire, parameter.type, tsType),
        wire: parameter.wire,
      });
    });
    const result = resultType(spec.result.wire, spec.result.type, tsType);
    return { spec, parameters, result, signature };
  });

  const imports = new Set(["NativeArguments", "allocateNativeModuleRequest", "awaitNativeModuleResult"]);
  for (const { spec } of functions) imports.add(decoderNames[spec.result.wire]);

  const lines: string[] = [];
  lines.push(
    `// Generated by @quickgui/cli from ${input.entryDisplayPath}. Do not edit: \`quickgui dev\`,`,
    "// `quickgui build`, and `quickgui modules` rewrite this file whenever the Zig source changes.",
    "/* eslint-disable */",
    "import {",
    ...[...imports].sort().map((identifier) => `  ${identifier},`),
    '} from "@quickgui/native/modules";',
    "",
    "// The module's C entry points. `@quickgui/cli` binds them through the scriptc FFI manifest when",
    "// it compiles the application; they are not callable under Bun or Node.",
    `declare function ${symbolPrefix}_call(index: number, args: Uint8Array, reply: (result: Uint8Array) => void): void;`,
    `declare function ${symbolPrefix}_call_async(request: number, index: number, args: Uint8Array): void;`,
    "",
    "function call(index: number, args: Uint8Array): Uint8Array {",
    "  let result: Uint8Array = new Uint8Array(0);",
    `  ${symbolPrefix}_call(index, args, (bytes: Uint8Array): void => {`,
    "    result = bytes;",
    "  });",
    "  return result;",
    "}",
    "",
    "function callAsync(index: number, args: Uint8Array): Promise<Uint8Array> {",
    "  const request = allocateNativeModuleRequest();",
    "  const result = awaitNativeModuleResult(request);",
    `  ${symbolPrefix}_call_async(request, index, args);`,
    "  return result;",
    "}",
  );

  for (const entry of declarationOrder) {
    lines.push("", ...typeDeclaration(entry, tsType));
  }

  functions.forEach(({ spec, parameters, result, signature }, index) => {
    const parameterList = parameters.map((parameter) => `${parameter.name}: ${parameter.type}`).join(", ");
    const encode = [
      `  const args = new NativeArguments(${parameters.length});`,
      ...parameters.map((parameter) => `  ${encodeStatement(parameter.wire, parameter.name)}`),
    ];
    const decoder = decoderNames[spec.result.wire];
    const decoded = (bytes: string): string => `${decoder}(${JSON.stringify(name)}, ${JSON.stringify(spec.name)}, ${bytes})`;
    const finish = (bytes: string): string => {
      switch (spec.result.wire) {
        case "void":
          return `  ${decoded(bytes)};`;
        case "json":
          return `  return JSON.parse(${decoded(bytes)}) as ${result};`;
        default:
          return `  return ${decoded(bytes)};`;
      }
    };
    const doc = signature?.doc ?? [];
    const zigLine = signature ? `Zig: \`${signature.text}\`` : undefined;
    lines.push("", ...jsDoc([...doc, ...(doc.length > 0 && zigLine ? [""] : []), ...(zigLine ? [zigLine] : [])]));
    lines.push(
      `export function ${spec.name}(${parameterList}): ${result} {`,
      ...encode,
      finish(`call(${index}, args.finish())`),
      "}",
    );
    lines.push(
      "",
      ...jsDoc([
        `Like \`${spec.name}\`, but runs on its own thread and resolves when it finishes.`,
        ...(zigLine ? ["", zigLine] : []),
      ]),
      `export async function ${spec.name}Async(${parameterList}): Promise<${result}> {`,
      ...encode,
      finish(`await callAsync(${index}, args.finish())`),
      "}",
    );
  });
  lines.push("");
  return lines.join("\n");
}

function encodeStatement(wire: NativeWire, parameterName: string): string {
  switch (wire) {
    case "void":
      return "args.nothing();";
    case "boolean":
      return `args.boolean(${parameterName});`;
    case "number":
      return `args.number(${parameterName});`;
    case "string":
      return `args.string(${parameterName});`;
    case "bytes":
      return `args.bytes(${parameterName});`;
    case "json":
      return `args.json(JSON.stringify(${parameterName}));`;
  }
}

function validateFunctionName(module: string, functionName: string, all: readonly NativeFunctionSpec[]): void {
  if (
    !identifierPattern.test(functionName) ||
    reservedIdentifiers.has(functionName) ||
    functionName.startsWith("quickgui_module_")
  ) {
    throw new CliError(
      `Native module ${module} exports \`${functionName}\`, which is not a usable JavaScript export name; rename it in ${NATIVE_MODULE_ENTRY}`,
    );
  }
  if (functionName.endsWith("Async")) {
    const base = functionName.slice(0, -"Async".length);
    if (all.some((spec) => spec.name === base)) {
      throw new CliError(
        `Native module ${module} exports both \`${base}\` and \`${functionName}\`; the generated \`${base}Async\` wrapper would clash. Rename one of them in ${NATIVE_MODULE_ENTRY}`,
      );
    }
  }
}

/** The TypeScript name for a Zig type name such as `main.DiffFile`, or nothing for anonymous types. */
export function hoistedName(zigName: string): string | undefined {
  if (/__(?:struct|union|enum|opaque)_\d+/.test(zigName)) return undefined;
  if (/[^A-Za-z0-9_.$]/.test(zigName)) return undefined;
  const short = zigName.slice(zigName.lastIndexOf(".") + 1);
  if (!identifierPattern.test(short) || reservedIdentifiers.has(short)) return undefined;
  return short;
}

function parameterType(
  wire: NativeWire,
  descriptor: NativeTypeDescriptor,
  tsType: (descriptor: NativeTypeDescriptor) => string,
): string {
  switch (wire) {
    case "void":
      return "undefined";
    case "boolean":
      return "boolean";
    case "number":
      return "number";
    case "string":
      return "string";
    case "bytes":
      return "Uint8Array";
    case "json":
      return tsType(descriptor);
  }
}

function resultType(
  wire: NativeWire,
  descriptor: NativeTypeDescriptor,
  tsType: (descriptor: NativeTypeDescriptor) => string,
): string {
  switch (wire) {
    case "void":
      return "void";
    case "boolean":
      return "boolean";
    case "number":
      return "number";
    case "string":
      return "string";
    case "bytes":
      return "Uint8Array";
    case "json":
      return tsType(descriptor);
  }
}

function typeDeclaration(
  entry: HoistedType,
  tsType: (descriptor: NativeTypeDescriptor) => string,
): string[] {
  const descriptor = entry.descriptor;
  const header = `/** Zig \`${entry.zigName}\`. */`;
  switch (descriptor.kind) {
    case "struct":
      return [header, `export interface ${entry.tsName} ${structLiteral(descriptor.fields, tsType, "\n")}`];
    case "enum":
      return [header, `export type ${entry.tsName} = ${enumLiteral(descriptor.values)};`];
    case "union":
      return [header, `export type ${entry.tsName} = ${unionLiteral(descriptor.variants, tsType)};`];
    default:
      return [];
  }
}

function structLiteral(
  fields: readonly { name: string; type: NativeTypeDescriptor; hasDefault: boolean }[],
  tsType: (descriptor: NativeTypeDescriptor) => string,
  separator: " " | "\n",
): string {
  if (fields.length === 0) return "Record<string, never>";
  const members = fields.map(
    (field) => `${propertyName(field.name)}${field.hasDefault ? "?" : ""}: ${tsType(field.type)};`,
  );
  if (separator === "\n") return `{\n${members.map((member) => `  ${member}`).join("\n")}\n}`;
  return `{ ${members.join(" ")} }`;
}

function enumLiteral(values: readonly string[]): string {
  return values.length === 0 ? "never" : values.map((value) => JSON.stringify(value)).join(" | ");
}

/**
 * A tagged union travels as `{ "<variant>": payload }`. It is typed as one record with an optional
 * field per variant rather than a union of records, which is what the static compiler can
 * represent for JSON values.
 */
function unionLiteral(
  variants: readonly { name: string; type: NativeTypeDescriptor }[],
  tsType: (descriptor: NativeTypeDescriptor) => string,
): string {
  return variants.length === 0
    ? "never"
    : `{ ${variants.map((variant) => `${propertyName(variant.name)}?: ${tsType(variant.type)};`).join(" ")} }`;
}

function propertyName(name: string): string {
  return identifierPattern.test(name) ? name : JSON.stringify(name);
}

function parenthesize(type: string): string {
  return type.includes(" | ") || type.includes(" & ") ? `(${type})` : type;
}

function jsDoc(lines: readonly string[]): string[] {
  const content = lines.map((line) => line.replaceAll("*/", "*\\/"));
  if (content.length === 0) return [];
  if (content.length === 1) return [`/** ${content[0]} */`];
  return ["/**", ...content.map((line) => (line.length > 0 ? ` * ${line}` : " *")), " */"];
}
