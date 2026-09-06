import { afterEach, beforeEach, expect, test } from "bun:test";
import { allocateRequest, rejectAllReplies, sendRequest, setAppContext, settleReply } from "./requests.ts";
import { Menu, dispatchSystemEvent, releaseMenuCallbacks } from "./system.ts";
import type { Window } from "./index.ts";

const commands: { app: number; acknowledgement: number; json: string }[] = [];
(globalThis as Record<string, unknown>).quickguiCommand = (app: number, acknowledgement: number, json: string) => {
  commands.push({ app, acknowledgement, json });
  return 0;
};
beforeEach(() => { commands.length = 0; setAppContext(7, true); });
afterEach(() => {
  rejectAllReplies(new Error("test finished"));
  releaseMenuCallbacks();
  setAppContext(0, false);
});

test("native request acceptance does not finish the operation", async () => {
  const request = allocateRequest();
  let finished = false;
  const reply = sendRequest(request, JSON.stringify({ method: "test", request }));
  void reply.then(() => { finished = true; });
  expect(commands).toHaveLength(1);
  expect(commands[0]!.acknowledgement).not.toBe(request);
  expect(commands[0]!.acknowledgement).not.toBe(0);
  settleReply(commands[0]!.acknowledgement, "null", undefined);
  await Promise.resolve();
  expect(finished).toBe(false);
  settleReply(request, '"complete"', undefined);
  expect(await reply).toBe('"complete"');
});

test("rejected native acceptance rejects the operation's Promise", async () => {
  const request = allocateRequest();
  const reply = sendRequest(request, JSON.stringify({ method: "test", request }));
  const failure = reply.catch((error: Error) => error);
  settleReply(commands[0]!.acknowledgement, undefined, "window closed");
  expect((await failure as Error).message).toBe("window closed");
});

test("popup callbacks survive acceptance and are released when native tracking ends", async () => {
  let clicks = 0;
  let flushed = false;
  const window = { closed: false, nativeId: 12, flush() { flushed = true; } } as unknown as Window;
  const pending = Menu.popup([{ label: "Run", click: () => { clicks += 1; } }], { window, x: 10, y: 20 });
  expect(flushed).toBe(true);
  const command = commands[0]!;
  const body = JSON.parse(command.json);
  const action = JSON.parse(body.menu)[0].items[0].id as number;
  settleReply(command.acknowledgement, "null", undefined);
  await Promise.resolve();
  dispatchSystemEvent("menu-action", action, undefined, undefined, undefined, undefined);
  expect(clicks).toBe(1);
  dispatchSystemEvent("popup-menu", body.request, "null", undefined, undefined, undefined);
  await pending;
  dispatchSystemEvent("menu-action", action, undefined, undefined, undefined, undefined);
  expect(clicks).toBe(1);
});

test("popup rejection releases callbacks and validates coordinates before queueing", async () => {
  let clicks = 0;
  const window = { closed: false, nativeId: 12, flush() {} } as unknown as Window;
  await expect(Menu.popup([], { window, x: 1 })).rejects.toThrow("both x and y");
  expect(commands).toHaveLength(0);
  const pending = Menu.popup([{ label: "Run", click: () => { clicks += 1; } }], { window });
  const failure = pending.catch((error: Error) => error);
  const command = commands[0]!;
  const action = JSON.parse(JSON.parse(command.json).menu)[0].items[0].id as number;
  settleReply(command.acknowledgement, undefined, "not available");
  expect((await failure as Error).message).toBe("not available");
  dispatchSystemEvent("menu-action", action, undefined, undefined, undefined, undefined);
  expect(clicks).toBe(0);
});
