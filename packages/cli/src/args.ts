import { CliError } from "./error.ts";
import { parseTarget, type QuickGuiTarget } from "./targets.ts";

export type ParsedCliCommand =
  | { command: "help"; topic?: "init" | "dev" | "build" }
  | { command: "version" }
  | {
      command: "init";
      directory: string;
      install: boolean;
      name?: string;
      identifier?: string;
    }
  | {
      command: "dev";
      project: string;
      configFile: string;
      once: boolean;
      launch: boolean;
      target?: QuickGuiTarget;
      signingIdentity?: string;
    }
  | {
      command: "build";
      project: string;
      configFile: string;
      target?: QuickGuiTarget;
      outDir?: string;
      signingIdentity?: string;
      notarizationProfile?: string;
    };

interface OptionSpec {
  key: string;
  value: boolean;
}

interface ParsedOptions {
  values: Map<string, string | true>;
  positionals: string[];
}

export function parseCliArgs(argv: string[]): ParsedCliCommand {
  if (argv.length === 0 || argv[0] === "--help" || argv[0] === "-h") {
    return { command: "help" };
  }
  if (argv[0] === "--version" || argv[0] === "-v") return { command: "version" };

  const command = argv[0];
  const rest = argv.slice(1);
  if (command === "help") {
    if (rest.length > 1 || (rest[0] && !["init", "dev", "build"].includes(rest[0]))) {
      throw new CliError("Usage: quickgui help [init|dev|build]");
    }
    return rest[0]
      ? { command: "help", topic: rest[0] as "init" | "dev" | "build" }
      : { command: "help" };
  }
  if (rest.includes("--help") || rest.includes("-h")) {
    if (command === "init" || command === "dev" || command === "build") {
      return { command: "help", topic: command };
    }
  }

  if (command === "init") {
    const parsed = parseOptions(rest, {
      "--name": { key: "name", value: true },
      "--identifier": { key: "identifier", value: true },
      "--no-install": { key: "noInstall", value: false },
    });
    if (parsed.positionals.length > 1) throw new CliError("Usage: quickgui init [directory]");
    const name = stringOption(parsed, "name");
    const identifier = stringOption(parsed, "identifier");
    return {
      command: "init",
      directory: parsed.positionals[0] ?? "quickgui-app",
      install: !parsed.values.has("noInstall"),
      ...(name ? { name } : {}),
      ...(identifier ? { identifier } : {}),
    };
  }

  if (command === "dev") {
    const parsed = parseOptions(rest, {
      "--project": { key: "project", value: true },
      "--config": { key: "configFile", value: true },
      "--target": { key: "target", value: true },
      "--sign": { key: "signingIdentity", value: true },
      "--once": { key: "once", value: false },
      "--no-launch": { key: "noLaunch", value: false },
    });
    rejectPositionals(parsed, "quickgui dev");
    const target = stringOption(parsed, "target");
    const signingIdentity = stringOption(parsed, "signingIdentity");
    return {
      command: "dev",
      project: stringOption(parsed, "project") ?? ".",
      configFile: stringOption(parsed, "configFile") ?? "quickgui.config.ts",
      once: parsed.values.has("once"),
      launch: !parsed.values.has("noLaunch"),
      ...(target ? { target: parseTarget(target) } : {}),
      ...(signingIdentity ? { signingIdentity } : {}),
    };
  }

  if (command === "build") {
    const parsed = parseOptions(rest, {
      "--project": { key: "project", value: true },
      "--config": { key: "configFile", value: true },
      "--target": { key: "target", value: true },
      "--out-dir": { key: "outDir", value: true },
      "--sign": { key: "signingIdentity", value: true },
      "--notarize": { key: "notarizationProfile", value: true },
    });
    rejectPositionals(parsed, "quickgui build");
    const target = stringOption(parsed, "target");
    const outDir = stringOption(parsed, "outDir");
    const signingIdentity = stringOption(parsed, "signingIdentity");
    const notarizationProfile = stringOption(parsed, "notarizationProfile");
    return {
      command: "build",
      project: stringOption(parsed, "project") ?? ".",
      configFile: stringOption(parsed, "configFile") ?? "quickgui.config.ts",
      ...(target ? { target: parseTarget(target) } : {}),
      ...(outDir ? { outDir } : {}),
      ...(signingIdentity ? { signingIdentity } : {}),
      ...(notarizationProfile ? { notarizationProfile } : {}),
    };
  }

  throw new CliError(`Unknown command: ${command}\nRun \`quickgui --help\` for usage.`);
}

function parseOptions(argv: string[], specs: Record<string, OptionSpec>): ParsedOptions {
  const values = new Map<string, string | true>();
  const positionals: string[] = [];
  let positionalOnly = false;

  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index]!;
    if (!positionalOnly && token === "--") {
      positionalOnly = true;
      continue;
    }
    if (positionalOnly || !token.startsWith("-")) {
      positionals.push(token);
      continue;
    }
    const equal = token.indexOf("=");
    const name = equal === -1 ? token : token.slice(0, equal);
    const inlineValue = equal === -1 ? undefined : token.slice(equal + 1);
    const spec = specs[name];
    if (!spec) throw new CliError(`Unknown option: ${name}`);
    if (values.has(spec.key)) throw new CliError(`Option may only be specified once: ${name}`);
    if (!spec.value) {
      if (inlineValue !== undefined) throw new CliError(`Option does not take a value: ${name}`);
      values.set(spec.key, true);
      continue;
    }
    const value = inlineValue ?? argv[++index];
    if (value === undefined || value.length === 0) {
      throw new CliError(`Option requires a value: ${name}`);
    }
    if (inlineValue === undefined && specs[value]) {
      throw new CliError(`Option requires a value: ${name}`);
    }
    values.set(spec.key, value);
  }

  return { values, positionals };
}

function stringOption(parsed: ParsedOptions, key: string): string | undefined {
  const value = parsed.values.get(key);
  return typeof value === "string" ? value : undefined;
}

function rejectPositionals(parsed: ParsedOptions, usage: string): void {
  if (parsed.positionals.length > 0) {
    throw new CliError(`${usage} does not accept positional arguments`);
  }
}
