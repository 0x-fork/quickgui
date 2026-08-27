import { existsSync } from "node:fs";
import { isAbsolute, resolve } from "node:path";
import { pathToFileURL } from "node:url";

import { CliError } from "./error.ts";
import { parseTarget, type QuickGuiTarget } from "./targets.ts";

export type { QuickGuiTarget } from "./targets.ts";

export interface MacOSConfig {
  minimumSystemVersion?: string;
  category?: string;
  icon?: string;
  signingIdentity?: string;
  entitlements?: string;
}

export interface WindowsConfig {
  icon?: string;
  publisher?: string;
  description?: string;
  copyright?: string;
  hideConsole?: boolean;
}

export interface QuickGuiConfig {
  name: string;
  identifier: string;
  version?: string;
  buildVersion?: string;
  entry?: string;
  outDir?: string;
  target?: QuickGuiTarget;
  resources?: string[];
  macos?: MacOSConfig;
  windows?: WindowsConfig;
}

export interface ResolvedQuickGuiConfig {
  name: string;
  executableName: string;
  identifier: string;
  version: string;
  buildVersion: string;
  entry: string;
  outDir: string;
  target?: QuickGuiTarget;
  resources: string[];
  macos: Required<Pick<MacOSConfig, "minimumSystemVersion" | "category">> & MacOSConfig;
  windows: Required<Pick<WindowsConfig, "hideConsole">> & WindowsConfig;
  projectRoot: string;
  configPath: string;
}

export function defineConfig(config: QuickGuiConfig): QuickGuiConfig {
  return config;
}

export async function loadConfig(
  projectRoot: string,
  configFile = "quickgui.config.ts",
): Promise<ResolvedQuickGuiConfig> {
  const root = resolve(projectRoot);
  const configPath = isAbsolute(configFile) ? configFile : resolve(root, configFile);
  if (!existsSync(configPath)) {
    throw new CliError(`QuickGUI config not found: ${configPath}`);
  }
  const url = pathToFileURL(configPath);
  url.searchParams.set("quickgui_reload", `${Date.now()}_${Math.random()}`);
  let module: { default?: unknown };
  try {
    module = (await import(url.href)) as { default?: unknown };
  } catch (error) {
    throw new CliError(`Could not load ${configPath}`, { cause: error });
  }
  return resolveConfig(module.default, root, configPath);
}

export function resolveConfig(
  input: unknown,
  projectRoot: string,
  configPath = resolve(projectRoot, "quickgui.config.ts"),
): ResolvedQuickGuiConfig {
  if (!isRecord(input)) throw new CliError("QuickGUI config must export an object");
  const name = requiredString(input.name, "name", 128);
  const identifier = requiredString(input.identifier, "identifier", 255);
  if (!/^[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+$/.test(identifier)) {
    throw new CliError(`Invalid application identifier \`${identifier}\``);
  }
  const version = optionalString(input.version, "version", 64) ?? "0.1.0";
  const buildVersion = optionalString(input.buildVersion, "buildVersion", 64) ?? version;
  const entry = resolveRelative(projectRoot, optionalString(input.entry, "entry", 1_024) ?? "src/app.tsx");
  const outDir = resolveRelative(projectRoot, optionalString(input.outDir, "outDir", 1_024) ?? "dist");
  const target = input.target === undefined ? undefined : parseTarget(requiredString(input.target, "target", 64));
  const resources = stringArray(input.resources, "resources").map((path) =>
    resolveRelative(projectRoot, path),
  );
  const macos = objectOrEmpty(input.macos, "macos");
  const windows = objectOrEmpty(input.windows, "windows");
  const icon = optionalString(macos.icon, "macos.icon", 1_024);
  const entitlements = optionalString(macos.entitlements, "macos.entitlements", 1_024);
  const windowsIcon = optionalString(windows.icon, "windows.icon", 1_024);

  return {
    name,
    executableName: executableName(name),
    identifier,
    version,
    buildVersion,
    entry,
    outDir,
    ...(target ? { target } : {}),
    resources,
    macos: {
      minimumSystemVersion:
        optionalString(macos.minimumSystemVersion, "macos.minimumSystemVersion", 32) ?? "13.0",
      category:
        optionalString(macos.category, "macos.category", 255) ??
        "public.app-category.developer-tools",
      ...(icon ? { icon: resolveRelative(projectRoot, icon) } : {}),
      ...(optionalString(macos.signingIdentity, "macos.signingIdentity", 512)
        ? { signingIdentity: String(macos.signingIdentity) }
        : {}),
      ...(entitlements ? { entitlements: resolveRelative(projectRoot, entitlements) } : {}),
    },
    windows: {
      hideConsole: optionalBoolean(windows.hideConsole, "windows.hideConsole") ?? true,
      ...(windowsIcon ? { icon: resolveRelative(projectRoot, windowsIcon) } : {}),
      ...(optionalString(windows.publisher, "windows.publisher", 255)
        ? { publisher: String(windows.publisher) }
        : {}),
      ...(optionalString(windows.description, "windows.description", 512)
        ? { description: String(windows.description) }
        : {}),
      ...(optionalString(windows.copyright, "windows.copyright", 512)
        ? { copyright: String(windows.copyright) }
        : {}),
    },
    projectRoot: resolve(projectRoot),
    configPath: resolve(configPath),
  };
}

function executableName(name: string): string {
  const value = name
    .normalize("NFKD")
    .replace(/[^A-Za-z0-9._-]+/g, "-")
    .replace(/^[._-]+|[._-]+$/g, "")
    .slice(0, 128);
  if (!value || value === "." || value === "..") {
    throw new CliError("Application name does not contain a usable executable name");
  }
  return value;
}

function resolveRelative(root: string, path: string): string {
  return isAbsolute(path) ? resolve(path) : resolve(root, path);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function objectOrEmpty(value: unknown, field: string): Record<string, unknown> {
  if (value === undefined) return {};
  if (!isRecord(value)) throw new CliError(`\`${field}\` must be an object`);
  return value;
}

function requiredString(value: unknown, field: string, maximum = 255): string {
  if (typeof value !== "string" || value.trim().length === 0 || value.length > maximum) {
    throw new CliError(`\`${field}\` must be a non-empty string of at most ${maximum} characters`);
  }
  return value.trim();
}

function optionalString(value: unknown, field: string, maximum: number): string | undefined {
  if (value === undefined) return undefined;
  return requiredString(value, field, maximum);
}

function optionalBoolean(value: unknown, field: string): boolean | undefined {
  if (value === undefined) return undefined;
  if (typeof value !== "boolean") throw new CliError(`\`${field}\` must be a boolean`);
  return value;
}

function stringArray(value: unknown, field: string): string[] {
  if (value === undefined) return [];
  if (!Array.isArray(value) || value.length > 1_024) {
    throw new CliError(`\`${field}\` must be an array with at most 1024 paths`);
  }
  return value.map((item, index) => requiredString(item, `${field}[${index}]`, 1_024));
}
