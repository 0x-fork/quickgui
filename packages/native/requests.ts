/**
 * Request plumbing shared by every native API.
 *
 * Commands and background integrations are fire-and-forget at the boundary; each one settles a
 * Promise when its reply event arrives on the application thread. CPU-only services answer
 * synchronously through the host's call-scoped reply callback.
 */

import { hostCall, hostCommand, hostInvoke } from "./ffi.ts";

class PendingReply {
  readonly resolve: (json: string) => void;
  readonly reject: (error: Error) => void;

  constructor(resolve: (json: string) => void, reject: (error: Error) => void) {
    this.resolve = resolve;
    this.reject = reject;
  }
}

class PendingData {
  readonly resolve: (data: Uint8Array) => void;
  readonly reject: (error: Error) => void;

  constructor(resolve: (data: Uint8Array) => void, reject: (error: Error) => void) {
    this.resolve = resolve;
    this.reject = reject;
  }
}

class PendingProgress {
  readonly listener: (json: string) => void;

  constructor(listener: (json: string) => void) {
    this.listener = listener;
  }
}

/** Reserved for the application readiness request. */
export const READY_REQUEST = 1;
let nextRequest = 2;
const pendingReplies = new Map<number, PendingReply>();
const pendingProgress = new Map<number, PendingProgress>();
const pendingData = new Map<number, PendingData>();
let appId = 0;
let appReady = false;

/** @internal Record the native application handle once it exists. */
export function setAppContext(id: number, ready: boolean): void {
  appId = id;
  appReady = ready;
}

export function nativeAppId(): number {
  return appId;
}

export function assertAppReady(): void {
  if (appId === 0 || !appReady) throw new Error("await app.whenReady() before using the native QuickGUI application");
}

/** Allocate one request id; ids never repeat while a process runs. */
export function allocateRequest(): number {
  const request = nextRequest;
  nextRequest += 1;
  if (nextRequest >= 0xffff_fff0) nextRequest = 2;
  return request;
}

/** Wait for the reply event carrying `request`; resolves with the JSON text of its value. */
export function awaitReply(request: number): Promise<string> {
  return new Promise<string>((resolve, reject) => {
    pendingReplies.set(request, new PendingReply(resolve, reject));
  });
}

/** Settle the reply registered for `request`, if any. */
export function settleReply(request: number, value: string | undefined, error: string | undefined): void {
  const pending = pendingReplies.get(request);
  pendingProgress.delete(request);
  if (pending === undefined) return;
  pendingReplies.delete(request);
  if (error !== undefined) pending.reject(new Error(error));
  else pending.resolve(value ?? "null");
}

/** Wait for the event carrying `request` and its binary payload, such as a native module result. */
export function awaitDataReply(request: number): Promise<Uint8Array> {
  return new Promise<Uint8Array>((resolve, reject) => {
    pendingData.set(request, new PendingData(resolve, reject));
  });
}

/** Settle the binary reply registered for `request`, if any. */
export function settleDataReply(request: number, data: Uint8Array | undefined, error: string | undefined): void {
  const pending = pendingData.get(request);
  if (pending === undefined) return;
  pendingData.delete(request);
  if (error !== undefined) pending.reject(new Error(error));
  else pending.resolve(data ?? new Uint8Array(0));
}

export function reportProgress(request: number, json: string): void {
  const progress = pendingProgress.get(request);
  if (progress !== undefined) progress.listener(json);
}

/** Reject every outstanding reply, when the application exits or is destroyed. */
export function rejectAllReplies(error: Error): void {
  const requests: number[] = [];
  for (const request of pendingReplies.keys()) requests.push(request);
  for (const request of requests) {
    const pending = pendingReplies.get(request);
    pendingReplies.delete(request);
    if (pending !== undefined) pending.reject(error);
  }
  pendingProgress.clear();
  const dataRequests: number[] = [];
  for (const request of pendingData.keys()) dataRequests.push(request);
  for (const request of dataRequests) {
    const pending = pendingData.get(request);
    pendingData.delete(request);
    if (pending !== undefined) pending.reject(error);
  }
}

/** Run one system command and resolve with the JSON text of its result. */
export function sendCommand(json: string): Promise<string> {
  if (appId === 0 || !appReady) {
    return Promise.reject(new Error("await app.whenReady() before using the native QuickGUI application"));
  }
  const request = allocateRequest();
  const reply = awaitReply(request);
  hostCommand(appId, request, json);
  return reply;
}

/**
 * Queue one system command whose own outcome arrives as a separate event carrying `request`.
 *
 * The command is acknowledged immediately; the returned Promise settles with that later event.
 */
export function sendRequest(request: number, json: string): Promise<string> {
  if (appId === 0 || !appReady) {
    return Promise.reject(new Error("await app.whenReady() before using the native QuickGUI application"));
  }
  const reply = awaitReply(request);
  void acceptRequest(request, json);
  return reply;
}

async function acceptRequest(request: number, json: string): Promise<void> {
  try {
    await sendCommand(json);
  } catch (error) {
    settleReply(request, undefined, error instanceof Error ? error.message : String(error));
  }
}

/** Queue one fire-and-forget system mutation. */
export function sendMutation(json: string): void {
  assertAppReady();
  hostCommand(appId, 0, json);
}

/** Run one background integration and resolve with the JSON text of its result. */
export function sendInvoke(method: string, params: string, onProgress: ((json: string) => void) | undefined): Promise<string> {
  const request = allocateRequest();
  const reply = awaitReply(request);
  if (onProgress !== undefined) pendingProgress.set(request, new PendingProgress(onProgress));
  hostInvoke(request, method, params);
  return reply;
}

interface SyncReply {
  ok: boolean;
  error?: string;
}

/** Answer one CPU-only service synchronously and return the JSON text of its value. */
export function callService(method: string, params: string): string {
  const reply = hostCall(method, params);
  const status = JSON.parse(reply) as SyncReply;
  if (!status.ok) throw new Error(status.error ?? "the native service failed");
  const prefix = '{"ok":true,"value":';
  if (!reply.startsWith(prefix) || !reply.endsWith("}")) throw new Error("the native service reply was malformed");
  return reply.slice(prefix.length, reply.length - 1);
}

/** Whether one JSON text encodes `null` or is absent. */
export function isNullJson(json: string | undefined): boolean {
  return json === undefined || json === "null" || json === "";
}

export function jsonBoolean(json: string): boolean {
  return json === "true";
}
