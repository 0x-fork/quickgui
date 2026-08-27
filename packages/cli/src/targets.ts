import { CliError } from "./error.ts";

export const supportedTargets = [
  "darwin-arm64",
  "darwin-x64",
  "linux-arm64",
  "linux-x64",
  "windows-arm64",
  "windows-x64",
] as const;

export type QuickGuiTarget = (typeof supportedTargets)[number];

export interface TargetInfo {
  target: QuickGuiTarget;
  platform: "darwin" | "linux" | "windows";
  architecture: "arm64" | "x64";
  bunTarget: Bun.Build.CompileTarget;
  nativeAddon: string;
}

const targets: Record<QuickGuiTarget, TargetInfo> = {
  "darwin-arm64": {
    target: "darwin-arm64",
    platform: "darwin",
    architecture: "arm64",
    bunTarget: "bun-darwin-arm64",
    nativeAddon: "quickgui-native.darwin-arm64.node",
  },
  "darwin-x64": {
    target: "darwin-x64",
    platform: "darwin",
    architecture: "x64",
    bunTarget: "bun-darwin-x64",
    nativeAddon: "quickgui-native.darwin-x64.node",
  },
  "linux-arm64": {
    target: "linux-arm64",
    platform: "linux",
    architecture: "arm64",
    bunTarget: "bun-linux-arm64",
    nativeAddon: "quickgui-native.linux-arm64-gnu.node",
  },
  "linux-x64": {
    target: "linux-x64",
    platform: "linux",
    architecture: "x64",
    bunTarget: "bun-linux-x64",
    nativeAddon: "quickgui-native.linux-x64-gnu.node",
  },
  "windows-arm64": {
    target: "windows-arm64",
    platform: "windows",
    architecture: "arm64",
    bunTarget: "bun-windows-arm64",
    nativeAddon: "quickgui-native.win32-arm64-msvc.node",
  },
  "windows-x64": {
    target: "windows-x64",
    platform: "windows",
    architecture: "x64",
    bunTarget: "bun-windows-x64",
    nativeAddon: "quickgui-native.win32-x64-msvc.node",
  },
};

export function parseTarget(value: string): QuickGuiTarget {
  if ((supportedTargets as readonly string[]).includes(value)) return value as QuickGuiTarget;
  throw new CliError(
    `Unsupported target \`${value}\`. Expected one of: ${supportedTargets.join(", ")}`,
  );
}

export function targetInfo(target: QuickGuiTarget): TargetInfo {
  return targets[target];
}

export function hostTarget(): QuickGuiTarget {
  const architecture = process.arch === "arm64" ? "arm64" : process.arch === "x64" ? "x64" : null;
  if (!architecture) throw new CliError(`Unsupported host architecture: ${process.arch}`);
  if (process.platform === "darwin") return `darwin-${architecture}`;
  if (process.platform === "linux") return `linux-${architecture}`;
  if (process.platform === "win32") return `windows-${architecture}`;
  throw new CliError(`Unsupported host platform: ${process.platform}`);
}
