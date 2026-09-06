/** Test adapter only. Application builds bind these calls directly through scriptc's FFI. */
import { afterAll, expect } from "bun:test";
import { dlopen, FFIType, JSCallback, ptr, toArrayBuffer, type Pointer } from "bun:ffi";
import { mkdirSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import { settleDataReply } from "../../../packages/native/requests.ts";

const root = resolve(import.meta.dir, "..");
const target = `darwin-${process.arch}`;
const archive = join(root, ".quickgui/modules/git", target, "libgit.a");
const outputDirectory = join(root, ".quickgui/tests");
mkdirSync(outputDirectory, { recursive: true });
const libraryPath = join(outputDirectory, "libgit-test.dylib");
const source = join(import.meta.dir, "module-host.c");
const modified = (path: string) => { try { return statSync(path).mtimeMs; } catch { return 0; } };
if (!modified(archive)) throw new Error("Build the test module first with `bun run modules` in examples/quick-git");
if (modified(libraryPath) < Math.max(modified(archive), modified(source))) {
  const build = Bun.spawnSync([process.env.QUICKGUI_ZIG ?? "zig", "cc", "-shared", "-o", libraryPath, source, archive], { stderr: "pipe" });
  if (build.exitCode !== 0) throw new Error(build.stderr.toString());
}
const library = dlopen(libraryPath, {
  test_call: { args: [FFIType.u32, FFIType.ptr, FFIType.u64, FFIType.ptr], returns: FFIType.void },
  test_call_async: { args: [FFIType.u32, FFIType.u32, FFIType.ptr, FFIType.u64], returns: FFIType.void },
  test_drain: { args: [FFIType.ptr], returns: FFIType.void },
});
const globals = globalThis as Record<string, unknown>;
const pending = new Set<number>();
let timer: ReturnType<typeof setTimeout> | undefined;
const completion = new JSCallback((id: number, data: Pointer, length: number | bigint) => {
  const copy = new Uint8Array(toArrayBuffer(data, 0, Number(length))).slice();
  pending.delete(id);
  settleDataReply(id, copy, undefined);
}, { args: [FFIType.u32, FFIType.ptr, FFIType.u64], returns: FFIType.void });
function drain() {
  timer = undefined;
  library.symbols.test_drain(completion.ptr);
  if (pending.size) timer = setTimeout(drain, 1);
}
globals.quickgui_module_git_call = (index: number, args: Uint8Array, reply: (result: Uint8Array) => void) => {
  const callback = new JSCallback((data: Pointer, length: number | bigint) => {
    reply(new Uint8Array(toArrayBuffer(data, 0, Number(length))).slice());
  }, { args: [FFIType.ptr, FFIType.u64, FFIType.ptr], returns: FFIType.void });
  try { library.symbols.test_call(index, args.length ? ptr(args) : null, args.length, callback.ptr); }
  finally { callback.close(); }
};
globals.quickgui_module_git_call_async = (request: number, index: number, args: Uint8Array) => {
  pending.add(request);
  library.symbols.test_call_async(request, index, args.length ? ptr(args) : null, args.length);
  if (timer === undefined) timer = setTimeout(drain, 1);
};
afterAll(() => { expect(pending.size).toBe(0); });
process.on("exit", () => {
  if (timer !== undefined) clearTimeout(timer);
  completion.close();
  library.close();
  delete globals.quickgui_module_git_call;
  delete globals.quickgui_module_git_call_async;
});
