/**
 * Compile a project's TypeScript into the source tree the native compiler consumes.
 *
 * The project entry, everything it imports, and the `@quickgui/native` and `@quickgui/ui`
 * packages are type-checked with the project's own JSX types by TypeScript 7, lowered, and
 * written into one flat tree of plain TypeScript modules with relative imports. The generated
 * `main.ts` embeds the packaged identity before the application entry runs.
 *
 * One TypeScript server is kept per project root, so the rebuilds of `quickgui dev` reuse its
 * parsed libraries and only re-read what changed.
 */

import { existsSync, mkdirSync, readFileSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { basename, dirname, isAbsolute, join, relative, resolve, sep } from "node:path";

import { API, DiagnosticCategory, type Diagnostic } from "typescript/unstable/async";

import { CliError } from "./error.ts";
import { LoweringError, lineAndColumn, lowerSourceFile } from "./jsx-lowering.ts";

export interface LowerProjectOptions {
  projectRoot: string;
  /** Absolute path of the application entry (`.ts` or `.tsx`). */
  entry: string;
  /** Directory that receives the lowered tree; it is recreated. */
  outDir: string;
  /** Packaged identity embedded ahead of the entry. */
  name: string;
  version: string;
  identifier: string;
  /** Font file paths, relative to the application's resource directory. */
  fonts: string[];
  /** Report TypeScript errors in the project's own sources instead of leaving that to the native compiler. */
  typeCheck?: boolean;
}

export interface LoweredProject {
  /** The generated program entry. */
  entryPath: string;
  /** Every written module. */
  files: string[];
  uiDir: string;
  nativeDir: string;
}

/** Locate an installed QuickGUI package, preferring the project's own dependency. */
export function resolvePackageDir(name: string, projectRoot: string): string {
  const candidates = [projectRoot, import.meta.dir];
  for (const from of candidates) {
    try {
      return realpathSync(dirname(Bun.resolveSync(`${name}/package.json`, from)));
    } catch {
      // Try the next location.
    }
  }
  throw new CliError(`Could not resolve the installed ${name} package`);
}

function isInside(directory: string, path: string): boolean {
  const rel = relative(directory, path);
  return rel !== "" && !rel.startsWith("..") && !isAbsolute(rel);
}

function toPosix(path: string): string {
  return path.split(sep).join("/");
}

function relativeSpecifier(from: string, to: string): string {
  let specifier = toPosix(relative(dirname(from), to));
  if (!specifier.startsWith(".")) specifier = `./${specifier}`;
  return specifier;
}

function outputExtension(path: string): string {
  if (path.endsWith(".tsx")) return `${path.slice(0, -4)}.ts`;
  if (path.endsWith(".mts")) return `${path.slice(0, -4)}.ts`;
  return path;
}

// --- TypeScript sessions -------------------------------------------------------------------------

interface Session {
  api: API;
  /** Config files this server has already opened; later builds only refresh them. */
  opened: Set<string>;
}

const sessions = new Map<string, Session>();

function session(projectRoot: string): Session {
  let current = sessions.get(projectRoot);
  if (current === undefined) {
    current = { api: new API({ cwd: projectRoot }), opened: new Set() };
    sessions.set(projectRoot, current);
  }
  return current;
}

/** Stop every TypeScript server this process started. */
export async function closeTypeScriptSessions(): Promise<void> {
  const open = [...sessions.values()];
  sessions.clear();
  for (const current of open) {
    try {
      await current.api.close();
    } catch {
      // A request still in flight is rejected when the connection closes; the server is gone.
    }
  }
}

/**
 * The tsconfig the lowering opens: the project's own configuration, when it has one, with the
 * entry as the only root and the options the native compiler depends on pinned.
 */
function writeLoweringConfig(projectRoot: string, entry: string, outDir: string): string {
  const projectConfig = findConfigFile(projectRoot);
  const hasNodeTypes = resolves("@types/node/package.json", projectRoot);
  const compilerOptions: Record<string, unknown> = {
    jsx: "preserve",
    jsxImportSource: "@quickgui/ui",
    noEmit: true,
    allowImportingTsExtensions: true,
    module: "preserve",
    moduleResolution: "bundler",
    target: "es2023",
    skipLibCheck: true,
    types: hasNodeTypes ? ["node"] : [],
    typeRoots: hasNodeTypes ? [dirname(dirname(Bun.resolveSync("@types/node/package.json", projectRoot)))] : [],
    outDir: undefined,
    rootDir: undefined,
    composite: undefined,
    declaration: undefined,
    incremental: undefined,
    tsBuildInfoFile: undefined,
  };
  if (projectConfig === undefined) {
    compilerOptions.strict = true;
    compilerOptions.lib = ["es2023"];
  }
  const config = {
    ...(projectConfig === undefined ? {} : { extends: projectConfig }),
    compilerOptions,
    files: [entry],
    include: [],
  };
  const configPath = join(dirname(outDir), `${basename(outDir)}.tsconfig.json`);
  mkdirSync(dirname(configPath), { recursive: true });
  const text = `${JSON.stringify(config, null, 2)}\n`;
  if (!existsSync(configPath) || readFileSync(configPath, "utf8") !== text) writeFileSync(configPath, text);
  return configPath;
}

function resolves(specifier: string, from: string): boolean {
  try {
    Bun.resolveSync(specifier, from);
    return true;
  } catch {
    return false;
  }
}

/** The nearest `tsconfig.json` at or above `projectRoot`. */
function findConfigFile(projectRoot: string): string | undefined {
  let directory = projectRoot;
  for (;;) {
    const candidate = join(directory, "tsconfig.json");
    if (existsSync(candidate)) return candidate;
    const parent = dirname(directory);
    if (parent === directory) return undefined;
    directory = parent;
  }
}

function formatDiagnostics(diagnostics: readonly Diagnostic[], projectRoot: string): string {
  const lines: string[] = [];
  for (const diagnostic of diagnostics) {
    let location = "";
    if (diagnostic.fileName !== undefined) {
      let position = "";
      try {
        const text = readFileSync(diagnostic.fileName, "utf8");
        const { line, column } = lineAndColumn(text, diagnostic.pos);
        position = `:${line}:${column}`;
      } catch {
        // The file is gone; report it without a position.
      }
      const shown = isInside(projectRoot, diagnostic.fileName) ? relative(projectRoot, diagnostic.fileName) : diagnostic.fileName;
      location = `${shown}${position} - `;
    }
    lines.push(`${location}error TS${diagnostic.code}: ${diagnostic.text}`);
  }
  return lines.join("\n");
}

interface OutputFile {
  /** The real path of the source. */
  file: string;
  /** Where the lowered module is written. */
  target: string;
}

export async function lowerProject(options: LowerProjectOptions): Promise<LoweredProject> {
  const projectRoot = realpathSync(options.projectRoot);
  const entry = realpathSync(options.entry);
  const uiDir = resolvePackageDir("@quickgui/ui", projectRoot);
  const nativeDir = resolvePackageDir("@quickgui/native", projectRoot);
  const uiIndexPath = join(uiDir, "index.ts");
  const outDir = resolve(options.outDir);
  const srcDir = join(outDir, "src");
  const configPath = writeLoweringConfig(projectRoot, entry, outDir);

  const current = session(projectRoot);
  const snapshot = await current.api.updateSnapshot(
    current.opened.has(configPath)
      ? { openProjects: [configPath], fileChanges: { invalidateAll: true } }
      : { openProjects: [configPath] },
  );
  current.opened.add(configPath);
  try {
    const project = snapshot.getProject(configPath);
    if (project === undefined) throw new CliError(`TypeScript could not open the project at ${configPath}`);
    const { program, checker } = project;

    // Map every compiled module to its output path first, so specifiers can be rewritten.
    const outputs = new Map<string, OutputFile>();
    for (const name of await program.getSourceFileNames()) {
      if (name.endsWith(".d.ts") || name.endsWith(".d.mts") || name.endsWith(".d.cts")) continue;
      const file = realpathSync(name);
      if (file === join(uiDir, "jsx-runtime.ts")) continue;
      let target: string;
      if (isInside(uiDir, file)) {
        target = join(srcDir, "pkg", "ui", outputExtension(relative(uiDir, file)));
      } else if (isInside(nativeDir, file)) {
        target = join(srcDir, "pkg", "native", outputExtension(relative(nativeDir, file)));
      } else if (isInside(projectRoot, file) && !toPosix(relative(projectRoot, file)).split("/").includes("node_modules")) {
        target = join(srcDir, "app", outputExtension(relative(projectRoot, file)));
      } else {
        throw new CliError(
          `${name} cannot be compiled to native code: only the project's own sources and the QuickGUI packages are compiled statically`,
        );
      }
      outputs.set(file.toLowerCase(), { file, target });
    }

    if (options.typeCheck === true) {
      // Only the project's own sources are checked: the QuickGUI packages are checked by their
      // own test suites, and checking them again on every rebuild would dominate the build time.
      const diagnostics: Diagnostic[] = [...(await program.getProgramDiagnostics()), ...(await program.getGlobalDiagnostics())];
      for (const output of outputs.values()) {
        if (!isInside(projectRoot, output.file) || isInside(uiDir, output.file) || isInside(nativeDir, output.file)) continue;
        diagnostics.push(...(await program.getSyntacticDiagnostics(output.file)), ...(await program.getSemanticDiagnostics(output.file)));
      }
      const errors = diagnostics.filter((diagnostic) => diagnostic.category === DiagnosticCategory.Error);
      if (errors.length > 0) throw new CliError(`Type errors\n${formatDiagnostics(errors, projectRoot)}`);
    }

    rmSync(outDir, { recursive: true, force: true });
    mkdirSync(srcDir, { recursive: true });
    const runtimeTarget = join(srcDir, "pkg", "ui", "runtime.ts");
    const reactiveTarget = join(srcDir, "pkg", "ui", "reactive.ts");
    const generatedTarget = join(srcDir, "pkg", "ui", "generated.ts");
    const files: string[] = [];
    for (const output of outputs.values()) {
      const source = await program.getSourceFile(output.file);
      if (source === undefined) throw new CliError(`TypeScript lost track of ${output.file}`);
      const target = output.target;
      let text: string;
      try {
        const lowered = await lowerSourceFile(source, {
          checker,
          uiIndexPath,
          runtimeSpecifier: relativeSpecifier(target, runtimeTarget),
          reactiveSpecifier: relativeSpecifier(target, reactiveTarget),
          generatedSpecifier: relativeSpecifier(target, generatedTarget),
          rewriteSpecifier: async (_specifier, literal) => {
            const symbol = await checker.getSymbolAtLocation(literal);
            const declaration = symbol === undefined ? undefined : symbol.declarations[0];
            if (declaration === undefined) return undefined;
            const resolved = outputs.get(String(declaration.path).toLowerCase());
            return resolved === undefined ? undefined : relativeSpecifier(target, resolved.target);
          },
        });
        text = lowered.text ?? source.text;
      } catch (error) {
        if (error instanceof LoweringError) throw new CliError(error.message);
        throw error;
      }
      mkdirSync(dirname(target), { recursive: true });
      writeFileSync(target, text);
      files.push(target);
    }

    const entryTarget = outputs.get(entry.toLowerCase())?.target;
    if (entryTarget === undefined) throw new CliError(`The entry ${entry} was not compiled`);
    const optionsPath = join(srcDir, "options.ts");
    const nativeIndex = join(srcDir, "pkg", "native", "index.ts");
    writeFileSync(
      optionsPath,
      `import { __embedApp } from ${JSON.stringify(relativeSpecifier(optionsPath, nativeIndex))};\n` +
        `__embedApp(${JSON.stringify(options.name)}, ${JSON.stringify(options.version)}, ${JSON.stringify(options.identifier)}, ${JSON.stringify(options.fonts)});\n`,
    );
    const mainPath = join(srcDir, "main.ts");
    writeFileSync(
      mainPath,
      `import ${JSON.stringify(relativeSpecifier(mainPath, optionsPath))};\n` +
        `import ${JSON.stringify(relativeSpecifier(mainPath, entryTarget))};\n`,
    );
    writeFileSync(join(srcDir, "package.json"), `${JSON.stringify({ name: "quickgui-app", type: "module", private: true }, null, 2)}\n`);
    writeFileSync(
      join(srcDir, "tsconfig.json"),
      `${JSON.stringify(
        {
          compilerOptions: {
            strict: true,
            target: "ES2023",
            module: "ESNext",
            moduleResolution: "bundler",
            allowImportingTsExtensions: true,
            noEmit: true,
            skipLibCheck: true,
            exactOptionalPropertyTypes: false,
          },
        },
        null,
        2,
      )}\n`,
    );
    files.push(optionsPath, mainPath);
    return { entryPath: mainPath, files, uiDir, nativeDir };
  } finally {
    await snapshot.dispose();
  }
}
