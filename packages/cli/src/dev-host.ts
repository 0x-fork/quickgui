import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { pathToFileURL } from "node:url";

import { plugin } from "bun";

interface DevelopmentManifest {
  version: 1;
  projectRoot: string;
  entry: string;
}

const manifest = readDevelopmentManifest();
const entry = process.env.QUICKGUI_ENTRY ?? manifest?.entry;
if (!entry) throw new Error("QUICKGUI_ENTRY is not set");

const projectRoot = process.env.QUICKGUI_PROJECT_ROOT ?? manifest?.projectRoot;
if (!projectRoot) throw new Error("QUICKGUI_PROJECT_ROOT is not set");
process.chdir(projectRoot);

const compilerPath = Bun.resolveSync("@quickgui/solid/compiler", projectRoot);
const { quickguiSolidPlugin } = await import(pathToFileURL(compilerPath).href);
plugin(quickguiSolidPlugin({ development: true }));

const entryUrl = pathToFileURL(resolve(entry));
entryUrl.searchParams.set("quickgui_launch", `${Date.now()}`);
await import(entryUrl.href);

function readDevelopmentManifest(): DevelopmentManifest | undefined {
  const path = resolve(dirname(process.execPath), "../Resources/quickgui-dev.json");
  if (!existsSync(path)) return undefined;
  const value = JSON.parse(readFileSync(path, "utf8")) as Partial<DevelopmentManifest>;
  if (
    value.version !== 1 ||
    typeof value.projectRoot !== "string" ||
    typeof value.entry !== "string"
  ) {
    throw new Error(`Invalid QuickGUI development manifest: ${path}`);
  }
  return value as DevelopmentManifest;
}
