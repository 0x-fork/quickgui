import {
  chmodSync,
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  renameSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, extname, join, relative, resolve } from "node:path";

import { quickguiSolidPlugin } from "@quickgui/solid/compiler";
import type { BunPlugin } from "bun";

import type { ResolvedQuickGuiConfig } from "./config.ts";
import { CliError, errorMessage } from "./error.ts";
import { targetInfo, type QuickGuiTarget } from "./targets.ts";

export type BuildMode = "development" | "production";

export interface BuildProjectOptions {
  mode: BuildMode;
  target: QuickGuiTarget;
  outDir?: string;
  signingIdentity?: string;
}

export interface BuildResult {
  artifactPath: string;
  executablePath: string;
  target: QuickGuiTarget;
  mode: BuildMode;
}

export const nativeExports = [
  "abortAppHost",
  "applyBatch",
  "applyHostedBatch",
  "closeWindow",
  "closeHostedWindow",
  "createApp",
  "createAnchoredWindow",
  "createHostedAnchoredWindow",
  "createHostedApp",
  "createHostedWindow",
  "createWindow",
  "destroyApp",
  "destroyHostedApp",
  "focusNode",
  "focusHostedNode",
  "protocolVersion",
  "pumpApp",
  "runAppHost",
  "showDialog",
  "showHostedDialog",
  "showHostedOpenDialog",
  "showHostedSaveDialog",
  "showOpenDialog",
  "showSaveDialog",
  "startApp",
  "startHostedApp",
  "takeEvents",
  "waitForHostedEvents",
] as const;

const nativeBindingShimSuffix = ".quickgui-binding-shim.js";

export async function buildProject(
  config: ResolvedQuickGuiConfig,
  options: BuildProjectOptions,
): Promise<BuildResult> {
  const info = targetInfo(options.target);
  validateInputs(config, info.platform);
  const baseOutDir = options.outDir
    ? resolve(config.projectRoot, options.outDir)
    : options.mode === "development"
      ? resolve(config.projectRoot, ".quickgui", "dev")
      : config.outDir;
  const targetOutDir = resolve(baseOutDir, options.target);
  mkdirSync(targetOutDir, { recursive: true });
  const stagingRoot = mkdtempSync(join(targetOutDir, ".quickgui-staging-"));

  try {
    const staged =
      info.platform === "darwin"
        ? await buildMacApp(config, options, stagingRoot)
        : await buildExecutable(config, options, stagingRoot);
    const finalPath = resolve(targetOutDir, basename(staged.artifactPath));
    replaceArtifact(staged.artifactPath, finalPath, stagingRoot);
    const executablePath = resolve(finalPath, relative(staged.artifactPath, staged.executablePath));
    return {
      artifactPath: finalPath,
      executablePath,
      target: options.target,
      mode: options.mode,
    };
  } finally {
    if (existsSync(stagingRoot)) rmSync(stagingRoot, { recursive: true, force: true });
  }
}

async function buildMacApp(
  config: ResolvedQuickGuiConfig,
  options: BuildProjectOptions,
  stagingRoot: string,
): Promise<BuildResult> {
  if (process.platform !== "darwin") {
    throw new CliError("macOS .app bundles must currently be assembled and signed on macOS");
  }
  const displayName = options.mode === "development" ? `${config.name} Dev` : config.name;
  const identifier =
    options.mode === "development" ? `${config.identifier}.dev` : config.identifier;
  const appPath = resolve(stagingRoot, `${config.executableName}.app`);
  const contents = resolve(appPath, "Contents");
  const macos = resolve(contents, "MacOS");
  const resources = resolve(contents, "Resources");
  mkdirSync(macos, { recursive: true });
  mkdirSync(resources, { recursive: true });
  const executablePath = resolve(macos, config.executableName);
  await compileExecutable(config, options, executablePath, stagingRoot);
  chmodSync(executablePath, 0o755);

  let iconFile: string | undefined;
  if (config.macos.icon) {
    if (extname(config.macos.icon).toLowerCase() !== ".icns") {
      throw new CliError("macos.icon must point to an .icns file");
    }
    iconFile = "AppIcon.icns";
  }
  const reservedResources = new Set<string>([
    ...(iconFile ? [iconFile] : []),
  ]);
  copyResources(config.resources, resources, reservedResources);
  if (config.macos.icon && iconFile) {
    cpSync(config.macos.icon, resolve(resources, iconFile));
  }
  writeFileSync(
    resolve(contents, "Info.plist"),
    macInfoPlist({
      name: config.name,
      displayName,
      executableName: config.executableName,
      identifier,
      version: config.version,
      buildVersion: config.buildVersion,
      minimumSystemVersion: config.macos.minimumSystemVersion,
      category: config.macos.category,
      ...(iconFile ? { iconFile } : {}),
    }),
  );
  writeFileSync(resolve(contents, "PkgInfo"), "APPL????");

  const identity = options.signingIdentity ?? config.macos.signingIdentity ?? "-";
  const signArguments = ["codesign", "--force", "--deep", "--sign", identity];
  if (config.macos.entitlements) {
    signArguments.push("--entitlements", config.macos.entitlements);
  }
  signArguments.push(appPath);
  await run(signArguments, config.projectRoot);
  await run(["codesign", "--verify", "--deep", "--strict", appPath], config.projectRoot);

  return {
    artifactPath: appPath,
    executablePath,
    target: options.target,
    mode: options.mode,
  };
}

async function buildExecutable(
  config: ResolvedQuickGuiConfig,
  options: BuildProjectOptions,
  stagingRoot: string,
): Promise<BuildResult> {
  const info = targetInfo(options.target);
  const suffix = info.platform === "windows" ? ".exe" : "";
  const executablePath = resolve(stagingRoot, `${config.executableName}${suffix}`);
  await compileExecutable(config, options, executablePath, stagingRoot);
  if (info.platform !== "windows") chmodSync(executablePath, 0o755);
  return {
    artifactPath: executablePath,
    executablePath,
    target: options.target,
    mode: options.mode,
  };
}

async function compileExecutable(
  config: ResolvedQuickGuiConfig,
  options: BuildProjectOptions,
  executablePath: string,
  stagingRoot: string,
): Promise<void> {
  const info = targetInfo(options.target);
  const windows =
    info.platform === "windows"
      ? {
          hideConsole: config.windows.hideConsole,
          title: config.name,
          version: windowsVersion(config.version),
          ...(config.windows.icon ? { icon: config.windows.icon } : {}),
          ...(config.windows.publisher ? { publisher: config.windows.publisher } : {}),
          ...(config.windows.description ? { description: config.windows.description } : {}),
          ...(config.windows.copyright ? { copyright: config.windows.copyright } : {}),
        }
      : undefined;

  let result: Bun.BuildOutput;
  try {
    const production = options.mode === "production";
    const hostEntrypoint = resolve(stagingRoot, "quickgui-app-host.ts");
    const workerEntrypoint = resolve(stagingRoot, "quickgui-app-worker.ts");
    const nativeHostModule = Bun.resolveSync("@quickgui/native/host", import.meta.dir);
    writeFileSync(
      hostEntrypoint,
      `import { runApplicationWorker } from ${JSON.stringify(nativeHostModule)};\n` +
        `const exitCode = await runApplicationWorker("./quickgui-app-worker.ts");\n` +
        `process.exit(exitCode);\n`,
    );
    writeFileSync(
      workerEntrypoint,
      `postMessage("quickgui:worker-ready");\n` +
        `import { reportWorkerFailure } from ${JSON.stringify(nativeHostModule)};\n` +
        `try {\n  await import(${JSON.stringify(config.entry)});\n} catch (error) {\n` +
        `  reportWorkerFailure(error);\n  throw error;\n}\n`,
    );
    result = await Bun.build({
      entrypoints: [hostEntrypoint, workerEntrypoint],
      throw: false,
      target: "bun",
      format: "esm",
      conditions: ["browser"],
      plugins: [
        quickguiSolidPlugin({ development: !production, projectRoot: config.projectRoot }),
        nativeBindingPlugin(info.nativeAddon, options.target),
      ],
      minify: production,
      sourcemap: production ? "none" : "inline",
      env: "disable",
      define: {
        "process.env.NODE_ENV": JSON.stringify(
          options.mode === "development" ? "development" : "production",
        ),
      },
      compile: {
        target: info.bunTarget,
        outfile: executablePath,
        autoloadDotenv: false,
        autoloadBunfig: false,
        autoloadPackageJson: !production,
        autoloadTsconfig: !production,
        ...(windows ? { windows } : {}),
      },
    });
  } catch (error) {
    throw new CliError(`Application compilation failed\n${errorMessage(error)}`);
  }
  if (!result.success) {
    throw new CliError(
      `Application compilation failed\n${result.logs.map((log) => String(log)).join("\n")}`,
    );
  }
}

export function nativeBindingPlugin(
  addonFile: string,
  target: QuickGuiTarget,
): BunPlugin {
  const packageRoots = new Map<string, string | undefined>();
  return {
    name: "quickgui-native-binding",
    setup(build) {
      build.onResolve({ filter: /^\.\/binding\.js$/ }, (arguments_) => {
        if (!arguments_.importer) return;
        let importer: string;
        try {
          importer = realpathSync(arguments_.importer);
        } catch {
          return;
        }
        let packageRoot = packageRoots.get(importer);
        if (!packageRoots.has(importer)) {
          packageRoot = findPackageRoot(importer, "@quickgui/native");
          packageRoots.set(importer, packageRoot);
        }
        if (!packageRoot) return;
        const addonPath = resolve(packageRoot, addonFile);
        if (!existsSync(addonPath)) {
          throw new CliError(
            `The installed @quickgui/native package does not contain ${addonFile}. ` +
              `Install a native package that supports ${target} or choose an available target.`,
          );
        }
        // Keep the generated JavaScript shim and the actual `.node` module at distinct module
        // identities. Reusing `addonPath` for both makes Bun resolve the shim's own `require()`
        // back to itself, producing a recursive initializer in the standalone executable.
        return {
          path: `${addonPath}${nativeBindingShimSuffix}`,
          namespace: "quickgui-native",
        };
      });
      build.onLoad({ filter: /.*/, namespace: "quickgui-native" }, ({ path }) => ({
        // Bun embeds directly required Node-API addons in standalone executables. The literal
        // target path must point at the real `.node` file, not this virtual shim.
        contents:
          `const nativeBinding = require(${JSON.stringify(path.slice(0, -nativeBindingShimSuffix.length))});\n` +
          `export const { ${nativeExports.join(", ")} } = nativeBinding;\n`,
        loader: "js",
      }));
    },
  };
}

function findPackageRoot(importer: string, expectedName: string): string | undefined {
  let directory = dirname(importer);
  for (;;) {
    const packageJson = resolve(directory, "package.json");
    if (existsSync(packageJson)) {
      try {
        const metadata = JSON.parse(readFileSync(packageJson, "utf8")) as { name?: unknown };
        if (metadata.name === expectedName) return directory;
      } catch {
        return undefined;
      }
    }
    const parent = dirname(directory);
    if (parent === directory) return undefined;
    directory = parent;
  }
}

function replaceArtifact(stagedPath: string, finalPath: string, stagingRoot: string): void {
  if (!existsSync(finalPath)) {
    renameSync(stagedPath, finalPath);
    return;
  }
  const backupPath = resolve(stagingRoot, ".quickgui-previous-artifact");
  renameSync(finalPath, backupPath);
  try {
    renameSync(stagedPath, finalPath);
  } catch (error) {
    renameSync(backupPath, finalPath);
    throw error;
  }
  rmSync(backupPath, { recursive: true, force: true });
}

function validateInputs(
  config: ResolvedQuickGuiConfig,
  platform: "darwin" | "linux" | "windows",
): void {
  if (!existsSync(config.entry) || !statSync(config.entry).isFile()) {
    throw new CliError(`Application entrypoint not found: ${config.entry}`);
  }
  for (const resource of config.resources) {
    if (!existsSync(resource)) throw new CliError(`Resource not found: ${resource}`);
  }
  if (platform === "darwin") {
    if (config.macos.icon && !existsSync(config.macos.icon)) {
      throw new CliError(`macOS icon not found: ${config.macos.icon}`);
    }
    if (config.macos.entitlements && !existsSync(config.macos.entitlements)) {
      throw new CliError(`macOS entitlements not found: ${config.macos.entitlements}`);
    }
  }
  if (platform === "windows" && config.windows.icon && !existsSync(config.windows.icon)) {
    throw new CliError(`Windows icon not found: ${config.windows.icon}`);
  }
}

function copyResources(
  paths: string[],
  destination: string,
  reservedNames: ReadonlySet<string> = new Set(),
): void {
  const names = new Map(
    [...reservedNames].map((name) => [name.toLocaleLowerCase("en-US"), name] as const),
  );
  for (const path of paths) {
    const name = basename(path);
    const normalizedName = name.toLocaleLowerCase("en-US");
    const previous = names.get(normalizedName);
    if (previous) {
      throw new CliError(
        `Resource destination name is reserved or duplicated: ${name} conflicts with ${previous}`,
      );
    }
    names.set(normalizedName, name);
    cpSync(path, resolve(destination, name), { recursive: statSync(path).isDirectory() });
  }
}

async function run(command: string[], cwd: string): Promise<void> {
  const child = Bun.spawn(command, {
    cwd,
    stdin: "ignore",
    stdout: "pipe",
    stderr: "pipe",
  });
  const [status, stdout, stderr] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);
  if (status !== 0) {
    const detail = stderr.trim() || stdout.trim();
    throw new CliError(`Command failed: ${command.join(" ")}${detail ? `\n${detail}` : ""}`);
  }
}

function windowsVersion(version: string): string {
  const parts = version
    .split(".")
    .slice(0, 4)
    .map((part) => (/^\d+$/.test(part) ? Number(part) : 0));
  while (parts.length < 4) parts.push(0);
  return parts.map((part) => Math.max(0, Math.min(65_535, part))).join(".");
}

interface MacInfoPlistOptions {
  name: string;
  displayName: string;
  executableName: string;
  identifier: string;
  version: string;
  buildVersion: string;
  minimumSystemVersion: string;
  category: string;
  iconFile?: string;
}

export function macInfoPlist(options: MacInfoPlistOptions): string {
  const icon = options.iconFile
    ? `\n  <key>CFBundleIconFile</key>\n  <string>${xml(options.iconFile)}</string>`
    : "";
  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleDisplayName</key>
  <string>${xml(options.displayName)}</string>
  <key>CFBundleExecutable</key>
  <string>${xml(options.executableName)}</string>${icon}
  <key>CFBundleIdentifier</key>
  <string>${xml(options.identifier)}</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>${xml(options.name)}</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>${xml(options.version)}</string>
  <key>CFBundleVersion</key>
  <string>${xml(options.buildVersion)}</string>
  <key>LSApplicationCategoryType</key>
  <string>${xml(options.category)}</string>
  <key>LSMinimumSystemVersion</key>
  <string>${xml(options.minimumSystemVersion)}</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
`;
}

function xml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&apos;");
}
