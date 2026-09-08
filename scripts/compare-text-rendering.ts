#!/usr/bin/env bun
/** Compare QuickGUI's actual GPU text against Electron's system-ui rasterization on macOS. */
import { mkdirSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

if (process.platform !== "darwin") throw new Error("This check compares macOS system fonts");
const root = resolve(import.meta.dir, "..");
const output = join(root, "target/desktop-benchmarks/text-reference");
mkdirSync(output, { recursive: true });
const text = "Open Completed — Issue inbox 0123456789";
const cases = [
  { size: 14, weight: 400, ink: "#20242c", background: "#f4f5f7", opacity: 1 },
  { size: 12, weight: 400, ink: "#788497", background: "#ffffff", opacity: 1 },
  { size: 14, weight: 400, ink: "#20242c", background: "#ffffff", opacity: 1 },
  { size: 18, weight: 400, ink: "#20242c", background: "#ffffff", opacity: 1 },
  { size: 24, weight: 700, ink: "#20242c", background: "#ffffff", opacity: 1 },
  { size: 14, weight: 500, ink: "#20242c", background: "#ffffff", opacity: 1 },
  { size: 14, weight: 600, ink: "#20242c", background: "#ffffff", opacity: 1 },
  { size: 14, weight: 400, ink: "#ffffff", background: "#111827", opacity: 1 },
  { size: 14, weight: 400, ink: "#c2410c", background: "#ffffff", opacity: 1 },
  { size: 14, weight: 400, ink: "#20242c", background: "#ffffff", opacity: 0.5 },
].map((value) => ({ ...value, text }));
writeFileSync(join(output, "cases.json"), JSON.stringify(cases));
writeFileSync(
  join(output, "index.html"),
  `<!doctype html><meta charset="utf-8"><style>
html,body { margin:0; width:640px; height:640px; }
section { height:64px; padding:16px; box-sizing:border-box; }
span { display:block; white-space:pre; font-family:-apple-system,BlinkMacSystemFont,sans-serif; line-height:32px; }
</style>${cases.map((c) => `<section style="background:${c.background}"><span style="font-size:${c.size}px;font-weight:${c.weight};color:${c.ink};opacity:${c.opacity}">${c.text}</span></section>`).join("")}`,
);
const entry = join(output, "electron.cjs");
writeFileSync(
  entry,
  `const {app,BrowserWindow}=require('electron');
const {writeFileSync}=require('node:fs');
const {join}=require('node:path');
process.env.QUICKGUI_TEXT_SCALE=process.argv.find(arg=>arg.startsWith('--quickgui-text-scale=')).split('=')[1];
app.commandLine.appendSwitch('force-device-scale-factor',process.env.QUICKGUI_TEXT_SCALE);
app.commandLine.appendSwitch('force-color-profile','srgb');
const log=(message)=>writeFileSync(join(__dirname,'capture-status.txt'),message);
app.setPath('userData',join(__dirname,'electron-profile'));
const watchdog=setTimeout(()=>{log('Timed out');app.exit(1)},20000);
log('Waiting for Electron');
app.whenReady().then(async()=>{
  log('Creating window');
  const win=new BrowserWindow({width:640,height:640,useContentSize:true,show:true,webPreferences:{backgroundThrottling:false}});
  log('Loading page');
  await win.loadFile(join(__dirname,'index.html'));
  log('Waiting for fonts');
  await win.webContents.executeJavaScript('document.fonts.ready.then(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))))');
  log('Capturing pixels');
  const capture=await win.webContents.capturePage();
  writeFileSync(join(__dirname,'electron-'+process.env.QUICKGUI_TEXT_SCALE+'.png'),capture.toPNG());
  log('Captured');
  clearTimeout(watchdog);
  app.exit(0);
}).catch(error=>{console.error(error);app.exit(1)});`,
);

async function run(argv: string[], env: Record<string, string | undefined> = {}) {
  const process = Bun.spawn(argv, {
    cwd: root,
    env: { ...Bun.env, ...env },
    stdout: "inherit",
    stderr: "inherit",
    ...(argv[0] === "/usr/bin/open" ? { timeout: 60_000 } : {}),
  });
  if ((await process.exited) !== 0) throw new Error(`Failed: ${argv.join(" ")}`);
}
for (const scale of [2]) {
  await run([
    "/usr/bin/open",
    "-W",
    "-n",
    "-a",
    join(root, "benchmarks/desktop/node_modules/electron/dist/Electron.app"),
    "--args",
    entry,
    `--quickgui-text-scale=${scale}`,
  ]);
}
await run(
  [
    "scripts/with-macos-ghostty-zig.sh",
    "cargo",
    "test",
    "--lib",
    "capture_text_rendering_comparison",
    "--",
    "--ignored",
    "--nocapture",
  ],
  {
    QUICKGUI_TEXT_REFERENCE_DIR: output,
  },
);
console.log(`Text comparison images: ${output}`);
