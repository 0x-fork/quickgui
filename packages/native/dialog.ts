import { statSync } from "node:fs";
import { basename, dirname, resolve, sep } from "node:path";
import * as binding from "./binding.js";

export type AlertDialogLevel = "info" | "warning" | "critical";
export type AlertDialogButtonRole = "default" | "cancel" | "other";

export interface AlertDialogButton {
  label: string;
  role?: AlertDialogButtonRole;
}

export interface AlertDialogOptions {
  level?: AlertDialogLevel;
  message: string;
  detail?: string;
  /** Defaults to one system-styled OK button. */
  buttons?: readonly (string | AlertDialogButton)[];
}

export type OpenDialogProperty =
  | "openFile"
  | "openDirectory"
  | "multiSelections"
  | "showHiddenFiles";

export interface FileDialogFilter {
  name: string;
  /** File extensions without a leading dot. Use `*` to match every file. */
  extensions: readonly string[];
}

export interface OpenDialogOptions {
  title?: string;
  /** Initial file or directory. */
  defaultPath?: string;
  filters?: readonly FileDialogFilter[];
  /** Custom open-button text. Currently supported by the macOS backend. */
  buttonLabel?: string;
  /** Defaults to `["openFile"]`. */
  properties?: readonly OpenDialogProperty[];
}

export interface OpenDialogResult {
  canceled: boolean;
  filePaths: string[];
}

export interface SaveDialogOptions {
  title?: string;
  /** Initial directory or complete suggested file path. */
  defaultPath?: string;
  filters?: readonly FileDialogFilter[];
  /** Custom save-button text. Currently supported by the macOS backend. */
  buttonLabel?: string;
  /** Currently supported by the macOS backend. */
  showHiddenFiles?: boolean;
}

export interface SaveDialogResult {
  canceled: boolean;
  filePath?: string;
}

export function normalizeAlertDialogOptions(
  options: AlertDialogOptions,
): binding.NativeDialogOptions {
  const normalized: binding.NativeDialogOptions = {
    message: options.message,
    buttons: (options.buttons ?? [{ label: "OK", role: "default" }]).map((button) => {
      if (typeof button === "string") return { label: button };
      const normalizedButton: binding.NativeDialogButton = { label: button.label };
      if (button.role !== undefined) normalizedButton.role = button.role;
      return normalizedButton;
    }),
  };
  if (options.level !== undefined) normalized.level = options.level;
  if (options.detail !== undefined) normalized.detail = options.detail;
  return normalized;
}

export function normalizeOpenDialogOptions(
  options: OpenDialogOptions,
): binding.NativeOpenDialogOptions {
  const properties = new Set(options.properties ?? ["openFile"]);
  const files = properties.has("openFile");
  const directories = properties.has("openDirectory");
  if (!files && !directories) {
    throw new TypeError("showOpenDialog properties must include openFile or openDirectory");
  }
  if (files && directories && process.platform !== "darwin") {
    throw new TypeError(
      "showOpenDialog cannot combine openFile and openDirectory on this platform",
    );
  }
  const normalized: binding.NativeOpenDialogOptions = {
    files,
    directories,
    multiple: properties.has("multiSelections"),
    filters: normalizeFileDialogFilters(options.filters),
    showsHiddenFiles: properties.has("showHiddenFiles"),
  };
  if (options.title !== undefined) normalized.title = options.title;
  if (options.buttonLabel !== undefined) normalized.prompt = options.buttonLabel;
  if (options.defaultPath !== undefined) {
    const defaultPath = resolve(options.defaultPath);
    if (options.defaultPath.endsWith(sep) || isExistingDirectory(defaultPath)) {
      normalized.directory = defaultPath;
    } else {
      normalized.directory = dirname(defaultPath);
      normalized.suggestedName = basename(defaultPath);
    }
  }
  return normalized;
}

export function normalizeSaveDialogOptions(
  options: SaveDialogOptions,
): binding.NativeSaveDialogOptions {
  const normalized: binding.NativeSaveDialogOptions = {
    directory: process.cwd(),
    filters: normalizeFileDialogFilters(options.filters),
    showsHiddenFiles: options.showHiddenFiles ?? false,
  };
  if (options.title !== undefined) normalized.title = options.title;
  if (options.buttonLabel !== undefined) normalized.prompt = options.buttonLabel;
  if (options.defaultPath !== undefined) {
    const defaultPath = resolve(options.defaultPath);
    if (options.defaultPath.endsWith(sep) || isExistingDirectory(defaultPath)) {
      normalized.directory = defaultPath;
    } else {
      normalized.directory = dirname(defaultPath);
      normalized.suggestedName = basename(defaultPath);
    }
  }
  return normalized;
}

function normalizeFileDialogFilters(
  filters: readonly FileDialogFilter[] | undefined,
): binding.NativeFileDialogFilter[] {
  return (filters ?? []).map((filter) => {
    if (filter.name.length === 0 || filter.extensions.length === 0) {
      throw new TypeError("file dialog filters require a name and at least one extension");
    }
    const extensions = filter.extensions.map((extension) => {
      if (
        extension.length === 0 ||
        extension.startsWith(".") ||
        extension.includes("\0") ||
        extension.includes("/") ||
        extension.includes("\\")
      ) {
        throw new TypeError(
          "file dialog filter extensions must be nonempty and omit dots and path separators",
        );
      }
      return extension;
    });
    return { name: filter.name, extensions };
  });
}

function isExistingDirectory(path: string): boolean {
  try {
    return statSync(path).isDirectory();
  } catch {
    return false;
  }
}
