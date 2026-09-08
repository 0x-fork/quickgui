import { createHash } from "node:crypto";
import {
  copyFileSync,
  existsSync,
  lstatSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  readlinkSync,
  realpathSync,
  rmSync,
  symlinkSync,
  utimesSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, isAbsolute, join, relative, resolve } from "node:path";
import { CliError } from "../error.ts";
import { createMoonbitParser } from "./parser.ts";
import { componentDefinitions, type Component } from "./components.ts";
import {
  identitySourceMap,
  originalPosition,
  transformMoonbit,
  uiAliases,
  packageImports,
  viewFunctions,
  type ViewSourceMap,
} from "./transform.ts";

const excluded = new Set([
  ".quickgui",
  ".git",
  ".mooncakes",
  ".repos",
  "node_modules",
  "_build",
  "target",
  "dist",
  "build",
]);
const hash = (value: string | Uint8Array) => createHash("sha256").update(value).digest("hex");
const revision = hash(
  [
    "workspace.ts",
    "transform.ts",
    "components.ts",
    "parser.ts",
    "api.generated.json",
    "parser/moonbit.wasm",
  ]
    .map((file) => hash(readFileSync(join(import.meta.dir, file))))
    .join("\n"),
);
interface SourceRecord {
  original: string;
  generated: string;
  hash: string;
  map: ViewSourceMap;
  bindings: number;
}
interface Cache {
  revision: string;
  files: SourceRecord[];
}
export interface MoonbitWorkspace {
  project: string;
  root: string;
  sources: SourceRecord[];
  bindings: number;
  changedFiles: number;
}

function writeChanged(filename: string, data: string) {
  // A package can gain its first UI import between builds. Never write a
  // transformed file through a previous source symlink into the user's checkout.
  try {
    if (lstatSync(filename).isSymbolicLink()) rmSync(filename);
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
  }
  if (existsSync(filename) && readFileSync(filename, "utf8") === data) return false;
  mkdirSync(dirname(filename), { recursive: true });
  writeFileSync(filename, data);
  return true;
}
function link(source: string, destination: string) {
  const sourceInfo = lstatSync(source);
  try {
    const destinationInfo = lstatSync(destination);
    if (destinationInfo.isSymbolicLink() && readlinkSync(destination) === source) return;
    if (
      process.platform === "win32" &&
      sourceInfo.isFile() &&
      destinationInfo.isFile() &&
      sourceInfo.size === destinationInfo.size &&
      Math.abs(sourceInfo.mtimeMs - destinationInfo.mtimeMs) < 1
    )
      return;
    rmSync(destination, { recursive: true, force: true });
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
  }
  mkdirSync(dirname(destination), { recursive: true });
  try {
    symlinkSync(source, destination, sourceInfo.isDirectory() ? "junction" : "file");
  } catch (error) {
    // File symlinks need Developer Mode or elevated privileges on Windows.
    if (
      process.platform !== "win32" ||
      !sourceInfo.isFile() ||
      !["EPERM", "EACCES"].includes((error as NodeJS.ErrnoException).code ?? "")
    )
      throw error;
    copyFileSync(source, destination);
    utimesSync(destination, sourceInfo.atime, sourceInfo.mtime);
  }
}
function findWorkspace(project: string): { root: string; members: string[] } {
  for (let path = project; ; path = dirname(path)) {
    const filename = join(path, "moon.work");
    if (existsSync(filename)) {
      // Current moon.work assignments are TOML-compatible. Reject unknown layouts
      // rather than compiling an unrelated registry version of a local module.
      const data = Bun.TOML.parse(readFileSync(filename, "utf8")) as { members?: unknown };
      if (!Array.isArray(data.members) || data.members.some((member) => typeof member !== "string"))
        throw new CliError(`Unsupported MoonBit workspace: ${filename}`);
      const members = data.members.map((member) => resolve(path, member as string));
      if (members.includes(project)) return { root: path, members };
      return { root: project, members: [project] };
    }
    if (dirname(path) === path) return { root: project, members: [project] };
  }
}

function moduleMetadata(directory: string) {
  const modern = join(directory, "moon.mod");
  const manifest = readFileSync(
    existsSync(modern) ? modern : join(directory, "moon.mod.json"),
    "utf8",
  );
  const name = existsSync(modern)
    ? /^\s*name\s*=\s*"([^\"]+)"/m.exec(manifest)?.[1]
    : (JSON.parse(manifest).name as string);
  // Module dependency strings are names or name@version in moon.mod, and keys
  // in moon.mod.json. Quoted tokens conservatively include local dependencies
  // without interpreting registry version requirements ourselves.
  const references = [...manifest.matchAll(/"(?:[^"\\]|\\.)*"/g)].map(
    (match) => JSON.parse(match[0]) as string,
  );
  return { directory, name, references };
}

/** Stable, disposable source mirrors preserve Moon's native incremental cache and never edit application files. */
export async function prepareMoonbitWorkspace(project: string): Promise<MoonbitWorkspace> {
  project = resolve(project);
  const originalWorkspace = findWorkspace(project);
  const root = join(project, ".quickgui", "moonbit-sources");
  const cacheFile = join(root, "sources.json");
  const cached: Cache = existsSync(cacheFile)
    ? JSON.parse(readFileSync(cacheFile, "utf8"))
    : { revision, files: [] };
  const previous = new Map(
    cached.revision === revision ? cached.files.map((file) => [file.original, file]) : [],
  );
  const sources: SourceRecord[] = [];
  const parser = await createMoonbitParser();
  let changedFiles = 0;
  const modules: string[] = [];
  let stagedProject = "";
  try {
    const members = originalWorkspace.members.map(moduleMetadata);
    const selected = new Set([project]);
    const queue = [members.find((member) => member.directory === project)!];
    for (const source of queue) {
      for (const dependency of members) {
        if (selected.has(dependency.directory) || !dependency.name) continue;
        if (
          source.references.some(
            (value) => value === dependency.name || value.startsWith(`${dependency.name}@`),
          )
        ) {
          selected.add(dependency.directory);
          queue.push(dependency);
        }
      }
    }
    const catalog = new Map<string, Record<string, Component>>();
    const packageComponents = new Map<string, Record<string, Component>>();
    const importedPackages = new Map<string, { path: string; alias: string }[]>();
    for (const member of members.filter(
      (member) => selected.has(member.directory) && member.name !== "egoist/quickgui",
    )) {
      const scan = (directory: string, inherited: { path: string; alias: string }[] = []) => {
        const manifest = ["moon.pkg", "moon.pkg.json"].find((file) =>
          existsSync(join(directory, file)),
        );
        const imports = manifest
          ? packageImports(
              parser,
              readFileSync(join(directory, manifest), "utf8"),
              join(directory, manifest),
            )
          : inherited;
        importedPackages.set(directory, imports);
        const aliases = imports
          .filter((item) => item.path === "egoist/quickgui/ui")
          .map((item) => item.alias);
        const definitions: Record<string, Component> = {};
        const entries = readdirSync(directory, { withFileTypes: true });
        for (const entry of entries) {
          if (excluded.has(entry.name)) continue;
          const path = join(directory, entry.name);
          if (entry.isFile() && entry.name.endsWith(".mbt") && aliases.length)
            Object.assign(
              definitions,
              componentDefinitions(parser, readFileSync(path, "utf8"), path, aliases),
            );
          else if (
            entry.isDirectory() &&
            !existsSync(join(path, "moon.mod")) &&
            !existsSync(join(path, "moon.mod.json"))
          )
            scan(path, imports);
        }
        packageComponents.set(directory, definitions);
        const suffix = relative(member.directory, directory).replaceAll("\\", "/");
        catalog.set(member.name + (suffix ? `/${suffix}` : ""), definitions);
      };
      scan(member.directory);
    }
    for (const { directory: member, name } of members.filter((member) =>
      selected.has(member.directory),
    )) {
      // The SDK implements the compiler targets and must not transform itself.
      if (name === "egoist/quickgui") {
        modules.push(member);
        continue;
      }
      const destination = join(
        root,
        member === project ? "project" : `module-${hash(member).slice(0, 12)}`,
      );
      modules.push(destination);
      if (member === project) stagedProject = destination;
      const expected = new Set<string>();
      const visit = (directory: string, output: string, inheritedAliases: string[] = []) => {
        mkdirSync(output, { recursive: true });
        let aliases = inheritedAliases;
        const pkg = ["moon.pkg", "moon.pkg.json"].find((file) => existsSync(join(directory, file)));
        if (pkg)
          aliases = uiAliases(
            parser,
            readFileSync(join(directory, pkg), "utf8"),
            join(directory, pkg),
          );
        const entries = readdirSync(directory, { withFileTypes: true });
        const functions = Object.assign(
          {},
          ...entries
            .filter((entry) => entry.isFile() && entry.name.endsWith(".mbt") && aliases.length)
            .map((entry) => {
              const input = join(directory, entry.name);
              return viewFunctions(parser, readFileSync(input, "utf8"), input, aliases);
            }),
        );
        const components = { ...packageComponents.get(directory) };
        for (const dependency of importedPackages.get(directory) ?? []) {
          for (const [name, component] of Object.entries(catalog.get(dependency.path) ?? {}))
            if (component.public) components[`@${dependency.alias}.${name}`] = component;
        }
        for (const entry of entries) {
          if (excluded.has(entry.name)) continue;
          const input = join(directory, entry.name),
            target = join(output, entry.name);
          expected.add(target);
          if (entry.isDirectory()) {
            // Independent nested modules (native extensions, fixtures) keep their
            // own build lifecycle and are not part of this Moon module.
            if (existsSync(join(input, "moon.mod")) || existsSync(join(input, "moon.mod.json")))
              link(input, target);
            else visit(input, target, aliases);
          } else if (
            entry.name.endsWith(".mbt") &&
            (aliases.length || Object.keys(components).length)
          ) {
            const source = readFileSync(input, "utf8");
            const digest = hash(JSON.stringify([aliases, functions, components]) + source);
            const old = previous.get(input);
            if (old?.hash === digest && old.generated === target && existsSync(target))
              sources.push(old);
            else {
              const transformed = transformMoonbit(
                parser,
                source,
                input,
                aliases,
                functions,
                components,
              );
              if (writeChanged(target, transformed.code)) changedFiles++;
              sources.push({
                original: input,
                generated: target,
                hash: digest,
                map: transformed.map,
                bindings: transformed.bindings,
              });
            }
          } else {
            link(input, target);
            if (/\.mbt$|^moon\.(pkg|mod)(\.json)?$/.test(entry.name)) {
              const source = readFileSync(input, "utf8");
              sources.push({
                original: input,
                generated: target,
                hash: hash(source),
                map: identitySourceMap(source),
                bindings: 0,
              });
            }
          }
        }
      };
      visit(member, destination);
      // Deletions and renamed packages must also invalidate Moon's source graph.
      const prune = (directory: string) => {
        for (const entry of readdirSync(directory, { withFileTypes: true })) {
          const path = join(directory, entry.name);
          if (!expected.has(path)) rmSync(path, { recursive: true, force: true });
          else if (entry.isDirectory()) prune(path);
        }
      };
      prune(destination);
    }
    if (!stagedProject)
      throw new CliError("The QuickGUI SDK itself cannot be compiled as an application");
    mkdirSync(root, { recursive: true });
    writeChanged(join(root, "moon.work"), `members = ${JSON.stringify(modules)}\n`);
    // Registry/git dependencies are Moon-owned; reuse their original caches.
    for (const cache of [".mooncakes", ".repos"]) {
      const path = join(originalWorkspace.root, cache);
      if (existsSync(path)) link(path, join(root, cache));
    }
    writeChanged(cacheFile, JSON.stringify({ revision, files: sources } satisfies Cache));
    return {
      root,
      project: stagedProject,
      sources,
      bindings: sources.reduce((sum, source) => sum + source.bindings, 0),
      changedFiles,
    };
  } finally {
    parser.delete();
  }
}

export function loadMoonbitSourceMaps(project: string): SourceRecord[] {
  const filename = join(project, ".quickgui/moonbit-sources/sources.json");
  if (!existsSync(filename)) return [];
  return (JSON.parse(readFileSync(filename, "utf8")) as Cache).files;
}

/** Handles compiler locations, test assertion locations, and symbolicated native stack frames. */
export function remapMoonbitDiagnostics(
  text: string,
  sources: SourceRecord[],
  cwd: string,
): string {
  for (const source of sources) {
    const paths = new Set([
      source.generated,
      relative(cwd, source.generated),
      source.generated.replaceAll("\\", "/"),
      ...(existsSync(source.generated) ? [realpathSync(source.generated)] : []),
    ]);
    const escaped = [...paths]
      .sort((a, b) => b.length - a.length)
      .map((path) => path.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"))
      .join("|");
    text = text.replace(
      new RegExp(`(?<![\\w/\\\\.-])(?:${escaped})(?::(\\d+)(?::(\\d+)(?:-(\\d+):(\\d+))?)?)?`, "g"),
      (_match, line, column, endLine, endColumn) => {
        if (!line) return source.original;
        const start = originalPosition(source.map, Number(line), Number(column ?? 1));
        const end = endLine
          ? originalPosition(source.map, Number(endLine), Number(endColumn))
          : undefined;
        return `${source.original}:${start.line}${column ? `:${start.column}` : ""}${end ? `-${end.line}:${end.column}` : ""}`;
      },
    );
  }
  return text;
}

/** Render structured compiler diagnostics with the original source excerpt and caret. */
export function renderMoonbitOutput(text: string, sources: SourceRecord[], cwd: string): string {
  return text
    .split("\n")
    .map((line) => {
      let diagnostic: {
        $message_type?: string;
        path?: string;
        loc?: string;
        level?: string;
        error_code?: number;
        message?: string;
      };
      try {
        diagnostic = JSON.parse(line);
      } catch {
        return remapMoonbitDiagnostics(line, sources, cwd);
      }
      if (diagnostic.$message_type !== "diagnostic" || !diagnostic.path || !diagnostic.loc)
        return remapMoonbitDiagnostics(line, sources, cwd);
      const location = remapMoonbitDiagnostics(
        `${diagnostic.path}:${diagnostic.loc}`,
        sources,
        cwd,
      );
      const match = /^(.*?):(\d+):(\d+)(?:-\d+:\d+)?$/.exec(location);
      let context = "";
      if (match && existsSync(match[1]!)) {
        const source = readFileSync(match[1]!, "utf8").split("\n")[Number(match[2]) - 1];
        if (source !== undefined)
          context = `\n  ${source}\n  ${" ".repeat(Math.min(Number(match[3]) - 1, source.length))}^`;
      }
      return `${location}: ${diagnostic.level ?? "error"}${diagnostic.error_code ? ` [${diagnostic.error_code}]` : ""}: ${diagnostic.message ?? ""}${context}`;
    })
    .join("\n");
}

export function stagedEntry(workspace: MoonbitWorkspace, project: string, entry: string) {
  const path = relative(project, entry);
  if (path.startsWith("..") || isAbsolute(path))
    throw new CliError("MoonBit entry must be inside the project directory");
  return join(workspace.project, path);
}
