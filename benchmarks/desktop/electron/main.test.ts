import { afterEach, expect, test } from "bun:test";
import { cpSync, mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createIssues } from "../workload";

const temporaryApps: string[] = [];
afterEach(() => {
  for (const path of temporaryApps.splice(0)) rmSync(path, { recursive: true, force: true });
});

async function packagedApp(withHTML: boolean, withScript = true) {
  const path = mkdtempSync(join(tmpdir(), "quickgui-electron-package-"));
  temporaryApps.push(path);
  const result = await Bun.build({
    entrypoints: [join(import.meta.dir, "main.ts")],
    outdir: path,
    target: "node",
    format: "cjs",
    external: ["electron"],
    minify: true,
  });
  expect(result.success).toBe(true);
  if (withHTML) cpSync(join(import.meta.dir, "../web/index.html"), join(path, "index.html"));
  if (withScript) {
    const web = await Bun.build({
      entrypoints: [join(import.meta.dir, "../web/app.ts")],
      outdir: path,
      target: "browser",
      minify: true,
      plugins: [
        {
          name: "benchmark-dataset",
          setup(build) {
            build.onResolve({ filter: /issues\.generated\.json$/ }, () => ({
              path: "issues",
              namespace: "dataset",
            }));
            build.onLoad({ filter: /.*/, namespace: "dataset" }, () => ({
              contents: JSON.stringify(createIssues()),
              loader: "json",
            }));
          },
        },
      ],
    });
    expect(web.success).toBe(true);
  }
  // Run the real bundled entry in an isolated installation. The Electron stub
  // only supplies its app path and verifies the requested asset exists there.
  const electron = join(path, "node_modules/electron");
  mkdirSync(electron, { recursive: true });
  writeFileSync(
    join(electron, "index.js"),
    `
    const { readFileSync } = require('node:fs');
    const { dirname, join } = require('node:path');
    exports.app = {
      whenReady: () => Promise.resolve(),
      getAppPath: () => ${JSON.stringify(path)},
      on() {}, quit() {}, exit: code => process.exit(code),
    };
    exports.BrowserWindow = class {
      async loadFile(path) {
        const html = readFileSync(path, 'utf8');
        if (!html.includes('id="search"') || !html.includes('id="notes"')) throw new Error('Missing issue tracker');
        const script = readFileSync(join(dirname(path), 'app.js'), 'utf8');
        if (!script.includes('APP-2000')) throw new Error('Missing issue dataset');
        console.log('Loaded issue tracker: ' + path);
      }
    };
  `,
  );
  const child = Bun.spawn([process.execPath, join(path, "main.js")], {
    cwd: path,
    stdout: "pipe",
    stderr: "pipe",
  });
  return {
    path,
    status: await child.exited,
    stdout: await new Response(child.stdout).text(),
    stderr: await new Response(child.stderr).text(),
  };
}

test("bundled Electron loads the issue tracker and dataset from its installed app path", async () => {
  const result = await packagedApp(true);
  expect(result.status).toBe(0);
  expect(result.stdout).toContain(`Loaded issue tracker: ${join(result.path, "index.html")}`);
});

test("missing packaged HTML exits with a failure instead of leaving a blank window", async () => {
  const result = await packagedApp(false);
  expect(result.status).toBe(1);
  expect(result.stderr).toContain("Could not load the benchmark issue tracker");
});

test("missing packaged script fails the packaging check", async () => {
  const result = await packagedApp(true, false);
  expect(result.status).toBe(1);
  expect(result.stderr).toContain("app.js");
});
