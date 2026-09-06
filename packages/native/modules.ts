/**
 * Runtime side of QuickGUI native modules.
 *
 * A native module is a Zig source file under `modules/<name>/main.zig` that `@quickgui/cli`
 * compiles into a static library with the runtime in `zig/quickgui.zig`, links into the
 * application, and exposes through a generated `modules/<name>/index.ts`. The generated file
 * declares the module's two C entry points for the scriptc FFI and uses this module to encode
 * arguments and decode results, so a module call is one native call with no JavaScript engine in
 * between.
 *
 * The encoding is deliberately small. Every value starts with a one-byte tag: booleans are one
 * byte, numbers are a little-endian `f64`, and strings, bytes, and JSON are a `u32` byte length
 * followed by the payload. A result starts with a status byte; a failed call carries the error
 * name instead of a value. Structured values (structs, arrays, optionals, enums, unions) travel
 * as JSON, so the application side only ever needs `JSON.parse` and `JSON.stringify`.
 */

import { allocateRequest, awaitDataReply } from "./requests.ts";

/** Encoding version shared with `zig/quickgui.zig`. */
export const NATIVE_MODULE_ABI = 2;

/** Most bytes accepted in one encoded argument list or result. */
export const MAX_NATIVE_MODULE_PAYLOAD_BYTES = 1024 * 1024 * 1024;

/** How one value crosses the boundary. */
export type NativeWire = "void" | "boolean" | "number" | "string" | "bytes" | "json";

const TAG_VOID = 0;
const TAG_BOOLEAN = 1;
const TAG_NUMBER = 2;
const TAG_STRING = 3;
const TAG_BYTES = 4;
const TAG_JSON = 5;

const STATUS_OK = 0;
const STATUS_ERROR = 1;

/** A call that failed inside the module: `code` is the Zig error name. */
export class NativeModuleError extends Error {
  readonly code: string;
  readonly module: string;
  readonly functionName: string;

  constructor(module: string, functionName: string, code: string) {
    super(module + "." + functionName + " failed with " + code);
    this.code = code;
    this.module = module;
    this.functionName = functionName;
  }
}

/** Encodes the arguments of one call; generated wrappers append one value per parameter. */
export class NativeArguments {
  _bytes: Uint8Array;
  _view: DataView;
  _length = 0;

  constructor(parameters: number) {
    this._bytes = new Uint8Array(parameters * 16 + 16);
    this._view = new DataView(this._bytes.buffer);
  }

  nothing(): void {
    this._ensure(1);
    this._bytes[this._length] = TAG_VOID;
    this._length += 1;
  }

  boolean(value: boolean): void {
    this._ensure(2);
    this._bytes[this._length] = TAG_BOOLEAN;
    this._bytes[this._length + 1] = value ? 1 : 0;
    this._length += 2;
  }

  number(value: number): void {
    this._ensure(9);
    this._bytes[this._length] = TAG_NUMBER;
    this._view.setFloat64(this._length + 1, value, true);
    this._length += 9;
  }

  string(value: string): void {
    this._sized(TAG_STRING, new TextEncoder().encode(value));
  }

  bytes(value: Uint8Array): void {
    this._sized(TAG_BYTES, value);
  }

  /** Append one value already serialized with `JSON.stringify`. */
  json(text: string): void {
    this._sized(TAG_JSON, new TextEncoder().encode(text));
  }

  finish(): Uint8Array {
    return this._bytes.subarray(0, this._length);
  }

  _sized(tag: number, payload: Uint8Array): void {
    if (payload.length > MAX_NATIVE_MODULE_PAYLOAD_BYTES) {
      throw new RangeError("native module arguments exceed " + String(MAX_NATIVE_MODULE_PAYLOAD_BYTES) + " bytes");
    }
    this._ensure(5 + payload.length);
    this._bytes[this._length] = tag;
    this._view.setUint32(this._length + 1, payload.length, true);
    this._bytes.set(payload, this._length + 5);
    this._length += 5 + payload.length;
  }

  _ensure(additional: number): void {
    const required = this._length + additional;
    if (required <= this._bytes.length) return;
    let capacity = this._bytes.length;
    while (capacity < required) capacity *= 2;
    const next = new Uint8Array(capacity);
    next.set(this._bytes.subarray(0, this._length));
    this._bytes = next;
    this._view = new DataView(next.buffer);
  }
}

function invalidResult(module: string, functionName: string): NativeModuleError {
  return new NativeModuleError(module, functionName, "InvalidResultEncoding");
}

function readU32(bytes: Uint8Array, offset: number): number {
  return (
    bytes[offset]! +
    bytes[offset + 1]! * 0x100 +
    bytes[offset + 2]! * 0x10000 +
    bytes[offset + 3]! * 0x1000000
  );
}

/** Check one result's status byte and value tag, throwing the module's error for a failed call. */
function checkResult(module: string, functionName: string, bytes: Uint8Array, tag: number): void {
  if (bytes.length < 1) throw invalidResult(module, functionName);
  if (bytes[0] === STATUS_ERROR) {
    if (bytes.length < 5) throw invalidResult(module, functionName);
    const length = readU32(bytes, 1);
    if (bytes.length < 5 + length) throw invalidResult(module, functionName);
    throw new NativeModuleError(module, functionName, new TextDecoder().decode(bytes.subarray(5, 5 + length)));
  }
  if (bytes[0] !== STATUS_OK || bytes.length < 2 || bytes[1] !== tag) throw invalidResult(module, functionName);
}

function sizedPayload(module: string, functionName: string, bytes: Uint8Array): Uint8Array {
  if (bytes.length < 6) throw invalidResult(module, functionName);
  const length = readU32(bytes, 2);
  if (bytes.length < 6 + length) throw invalidResult(module, functionName);
  return bytes.subarray(6, 6 + length);
}

export function decodeNativeVoid(module: string, functionName: string, bytes: Uint8Array): void {
  checkResult(module, functionName, bytes, TAG_VOID);
}

export function decodeNativeBoolean(module: string, functionName: string, bytes: Uint8Array): boolean {
  checkResult(module, functionName, bytes, TAG_BOOLEAN);
  if (bytes.length < 3) throw invalidResult(module, functionName);
  return bytes[2] !== 0;
}

export function decodeNativeNumber(module: string, functionName: string, bytes: Uint8Array): number {
  checkResult(module, functionName, bytes, TAG_NUMBER);
  if (bytes.length < 10) throw invalidResult(module, functionName);
  return new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getFloat64(2, true);
}

export function decodeNativeString(module: string, functionName: string, bytes: Uint8Array): string {
  checkResult(module, functionName, bytes, TAG_STRING);
  return new TextDecoder().decode(sizedPayload(module, functionName, bytes));
}

export function decodeNativeBytes(module: string, functionName: string, bytes: Uint8Array): Uint8Array {
  checkResult(module, functionName, bytes, TAG_BYTES);
  const payload = sizedPayload(module, functionName, bytes);
  const copy = new Uint8Array(payload.length);
  copy.set(payload);
  return copy;
}

/** Return the JSON text of one structured result; the generated wrapper parses it. */
export function decodeNativeJson(module: string, functionName: string, bytes: Uint8Array): string {
  checkResult(module, functionName, bytes, TAG_JSON);
  return new TextDecoder().decode(sizedPayload(module, functionName, bytes));
}

/** @internal Allocate the request id an asynchronous module call reports its result under. */
export function allocateNativeModuleRequest(): number {
  return allocateRequest();
}

/** @internal Resolve with the encoded result the host delivers for `request`. */
export function awaitNativeModuleResult(request: number): Promise<Uint8Array> {
  return awaitDataReply(request);
}
