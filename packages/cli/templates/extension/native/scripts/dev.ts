import { buildExtension, root } from "./build.ts";

// Build the extension once. The QuickGUI watcher then recompiles only Go edits.
const directory = await buildExtension();
const child = Bun.spawn(
  [process.execPath, "run", "--bun", "quickgui", "dev", ...process.argv.slice(2)],
  {
    cwd: root,
    env: { ...process.env, CGO_ENABLED: "0", QUICKGUI_EXTENSION_DIR: directory },
    stdin: "inherit",
    stdout: "inherit",
    stderr: "inherit",
  },
);
process.exitCode = await child.exited;
