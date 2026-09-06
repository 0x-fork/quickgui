import { expect, test } from "bun:test";
import { runProcess } from "./process.ts";
const cwd = process.cwd();

test("process stdin preserves binary bytes and shell-looking arguments", async () => {
  const input = new Uint8Array([0, 10, 127, 255, 65]);
  const result = await runProcess(["/bin/cat"], { cwd, stdin: input });
  expect(result.stdout).toEqual(input);
  expect(result.exitCode).toBe(0);
  const literal = 'one "quote"; $(exit 9) `exit 8`';
  const args = await runProcess(["/usr/bin/printf", "%s", literal], { cwd, stdin: "" });
  expect(new TextDecoder().decode(args.stdout)).toBe(literal);
});

test("process waits for both output streams and preserves failure status", async () => {
  const result = await runProcess(["/bin/sh", "-c", 'printf out; printf err >&2; exit 7'], { cwd });
  expect(new TextDecoder().decode(result.stdout)).toBe("out");
  expect(result.stderr).toBe("err");
  expect(result.exitCode).toBe(7);
});

test("process enforces output bounds and deadlines", async () => {
  const bounded = await runProcess(["/usr/bin/yes"], { cwd, maxOutputBytes: 17 });
  expect(bounded.truncated).toBe(true);
  expect(bounded.stdout.byteLength).toBe(17);
  const timeout = await runProcess(["/bin/sleep", "10"], { cwd, timeoutMs: 30 });
  expect(timeout.timedOut).toBe(true);
});

test("process cancellation handles already-aborted and active requests", async () => {
  const controller = new AbortController();
  controller.abort();
  expect((await runProcess(["/bin/sleep", "10"], { cwd, signal: controller.signal })).aborted).toBe(true);
  const active = new AbortController();
  const result = runProcess(["/bin/sleep", "10"], { cwd, signal: active.signal });
  setTimeout(() => active.abort(), 25);
  expect((await result).aborted).toBe(true);
});
