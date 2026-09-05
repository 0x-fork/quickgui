/**
 * Runtime side of QuickGUI native modules.
 *
 * A native module is a Zig source file under `modules/<name>/main.zig` that `@quickgui/cli`
 * compiles into a Node-API addon with the runtime in `zig/quickgui.zig`. The addon exposes three
 * functions — `manifest()`, `call(index, bytes)`, and `callAsync(index, bytes)` — and this module
 * turns them into ordinary typed JavaScript calls: it encodes arguments into the shared binary
 * format, decodes results, and throws the Zig error name when a call fails.
 *
 * The encoding is deliberately small. Every value starts with a one-byte tag: booleans are one
 * byte, numbers are a little-endian `f64`, and strings, bytes, and JSON are a `u32` byte length
 * followed by the payload. A result starts with a status byte; a failed call carries the error
 * name instead of a value. Structured values (structs, arrays, optionals, enums, unions) travel
 * as JSON, so the JavaScript side only ever needs `JSON.parse` and `JSON.stringify`.
 */

import { Buffer } from "node:buffer";

/** Encoding version shared with `zig/quickgui.zig`. */
export const NATIVE_MODULE_ABI = 1;

/** Most bytes accepted in one encoded argument list or result. */
export const MAX_NATIVE_MODULE_PAYLOAD_BYTES = 1024 * 1024 * 1024;

export type NativeWire = "void" | "boolean" | "number" | "string" | "bytes" | "json";

const wireTags: Record<NativeWire, number> = {
  void: 0,
  boolean: 1,
  number: 2,
  string: 3,
  bytes: 4,
  json: 5,
};

const STATUS_OK = 0;
const STATUS_ERROR = 1;

/** Description of one Zig type, as emitted by the module runtime's manifest. */
export type NativeTypeDescriptor =
  | { kind: "void" }
  | { kind: "boolean" }
  | { kind: "number" }
  | { kind: "string" }
  | { kind: "bytes" }
  | { kind: "any" }
  | { kind: "optional"; inner: NativeTypeDescriptor }
  | { kind: "array"; element: NativeTypeDescriptor }
  | { kind: "tuple"; elements: NativeTypeDescriptor[] }
  | { kind: "struct"; name: string; fields: NativeStructField[] }
  | { kind: "enum"; name: string; values: string[] }
  | { kind: "union"; name: string; variants: NativeUnionVariant[] }
  | { kind: "ref"; name: string };

export interface NativeStructField {
  name: string;
  type: NativeTypeDescriptor;
  /** Whether the Zig field has a default value, so JavaScript may omit it. */
  hasDefault: boolean;
}

export interface NativeUnionVariant {
  name: string;
  type: NativeTypeDescriptor;
}

export interface NativeValueSpec {
  wire: NativeWire;
  type: NativeTypeDescriptor;
}

/** A parameter the runtime supplies itself instead of reading from JavaScript. */
export interface NativeInjectedParameter {
  injected: "allocator";
}

export type NativeParameterSpec = NativeValueSpec | NativeInjectedParameter;

export interface NativeFunctionSpec {
  name: string;
  params: NativeParameterSpec[];
  result: NativeValueSpec;
}

/** The manifest a compiled module reports from `manifest()`. */
export interface NativeModuleManifest {
  abi: number;
  /** Version of the Zig compiler that built the module. */
  zig: string;
  functions: NativeFunctionSpec[];
}

/** The part of a manifest the runtime needs to encode calls; embedded in generated `index.ts`. */
export interface NativeModuleBinding {
  abi: number;
  functions: NativeFunctionBinding[];
}

export interface NativeFunctionBinding {
  name: string;
  params: (NativeWire | "allocator")[];
  result: NativeWire;
}

/** The exports of a compiled native module addon. */
export interface NativeModuleAddon {
  manifest(): string;
  call(index: number, encodedArguments: Uint8Array): Uint8Array;
  callAsync(index: number, encodedArguments: Uint8Array): Promise<Uint8Array>;
}

/** A call that failed inside the module: `code` is the Zig error name. */
export class NativeModuleError extends Error {
  override readonly name = "NativeModuleError";
  readonly code: string;
  readonly module: string;
  readonly functionName: string;

  constructor(module: string, functionName: string, code: string) {
    super(`${module}.${functionName} failed with ${code}`);
    this.code = code;
    this.module = module;
    this.functionName = functionName;
  }
}

/** Parse and validate the JSON a module reports from `manifest()`. */
export function parseNativeModuleManifest(text: string): NativeModuleManifest {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch (error) {
    throw new Error("The native module manifest is not valid JSON", { cause: error });
  }
  if (!isRecord(parsed) || typeof parsed.abi !== "number" || !Array.isArray(parsed.functions)) {
    throw new Error("The native module manifest does not have the expected shape");
  }
  if (parsed.abi !== NATIVE_MODULE_ABI) {
    throw new Error(
      `The native module was built for ABI ${parsed.abi}, but this @quickgui/native expects ABI ${NATIVE_MODULE_ABI}`,
    );
  }
  const functions = parsed.functions.map((entry, index): NativeFunctionSpec => {
    if (
      !isRecord(entry) ||
      typeof entry.name !== "string" ||
      !Array.isArray(entry.params) ||
      !isValueSpec(entry.result)
    ) {
      throw new Error(`Native module function ${index} has an invalid manifest entry`);
    }
    const params = entry.params.map((parameter): NativeParameterSpec => {
      if (isRecord(parameter) && parameter.injected === "allocator") return { injected: "allocator" };
      if (isValueSpec(parameter)) return parameter;
      throw new Error(`Native module function ${entry.name} has an invalid parameter entry`);
    });
    return { name: entry.name, params, result: entry.result };
  });
  return {
    abi: parsed.abi,
    zig: typeof parsed.zig === "string" ? parsed.zig : "",
    functions,
  };
}

/** Reduce a manifest to the wire-level signature the runtime needs. */
export function nativeModuleBinding(manifest: NativeModuleManifest): NativeModuleBinding {
  return {
    abi: manifest.abi,
    functions: manifest.functions.map((spec) => ({
      name: spec.name,
      params: spec.params.map((parameter) => ("injected" in parameter ? "allocator" : parameter.wire)),
      result: spec.result.wire,
    })),
  };
}

function bindingSignature(binding: NativeModuleBinding): string {
  return binding.functions
    .map((spec) => `${spec.name}(${spec.params.join(",")})${spec.result}`)
    .join(";");
}

/** Encode one call's arguments for `call`/`callAsync`. */
export function encodeNativeArguments(
  spec: NativeFunctionBinding,
  values: readonly unknown[],
  module = "module",
): Uint8Array {
  const parameters = spec.params.filter((wire) => wire !== "allocator") as NativeWire[];
  if (values.length !== parameters.length) {
    throw new TypeError(
      `${module}.${spec.name} expects ${parameters.length} argument${parameters.length === 1 ? "" : "s"}, got ${values.length}`,
    );
  }
  const payloads: (Uint8Array | string | number | boolean | undefined)[] = new Array(parameters.length);
  const lengths = new Uint32Array(parameters.length);
  let total = 0;
  parameters.forEach((wire, index) => {
    const value = values[index];
    const describe = (expected: string): TypeError =>
      new TypeError(`${module}.${spec.name}: argument ${index + 1} must be ${expected}`);
    switch (wire) {
      case "void":
        throw describe("absent");
      case "boolean":
        if (typeof value !== "boolean") throw describe("a boolean");
        payloads[index] = value;
        total += 2;
        return;
      case "number":
        if (typeof value !== "number") throw describe("a number");
        payloads[index] = value;
        total += 9;
        return;
      case "string": {
        if (typeof value === "string") {
          const length = Buffer.byteLength(value, "utf8");
          payloads[index] = value;
          lengths[index] = length;
          total += 5 + length;
          return;
        }
        if (value instanceof Uint8Array) {
          payloads[index] = value;
          lengths[index] = value.byteLength;
          total += 5 + value.byteLength;
          return;
        }
        throw describe("a string or a Uint8Array");
      }
      case "bytes":
        if (!(value instanceof Uint8Array)) throw describe("a Uint8Array");
        payloads[index] = value;
        lengths[index] = value.byteLength;
        total += 5 + value.byteLength;
        return;
      case "json": {
        const text = value === undefined ? undefined : JSON.stringify(value);
        if (text === undefined) throw describe("a JSON-serializable value");
        const length = Buffer.byteLength(text, "utf8");
        payloads[index] = text;
        lengths[index] = length;
        total += 5 + length;
        return;
      }
    }
  });
  if (total > MAX_NATIVE_MODULE_PAYLOAD_BYTES) {
    throw new RangeError(`${module}.${spec.name}: arguments exceed ${MAX_NATIVE_MODULE_PAYLOAD_BYTES} bytes`);
  }
  const buffer = Buffer.allocUnsafe(total);
  let offset = 0;
  parameters.forEach((wire, index) => {
    const payload = payloads[index];
    buffer[offset] = wireTags[wire];
    offset += 1;
    switch (wire) {
      case "void":
        return;
      case "boolean":
        buffer[offset] = payload ? 1 : 0;
        offset += 1;
        return;
      case "number":
        buffer.writeDoubleLE(payload as number, offset);
        offset += 8;
        return;
      case "string":
      case "bytes":
      case "json": {
        const length = lengths[index]!;
        buffer.writeUInt32LE(length, offset);
        offset += 4;
        if (typeof payload === "string") buffer.write(payload, offset, length, "utf8");
        else buffer.set(payload as Uint8Array, offset);
        offset += length;
      }
    }
  });
  return buffer;
}

/** Decode a `call`/`callAsync` result, throwing the module's error when the call failed. */
export function decodeNativeResult(
  spec: NativeFunctionBinding,
  bytes: Uint8Array,
  module = "module",
): unknown {
  const view = Buffer.isBuffer(bytes) ? bytes : Buffer.from(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const invalid = (): NativeModuleError => new NativeModuleError(module, spec.name, "InvalidResultEncoding");
  if (view.byteLength < 1) throw invalid();
  if (view[0] === STATUS_ERROR) {
    if (view.byteLength < 5) throw invalid();
    const length = view.readUInt32LE(1);
    if (view.byteLength < 5 + length) throw invalid();
    throw new NativeModuleError(module, spec.name, view.toString("utf8", 5, 5 + length));
  }
  if (view[0] !== STATUS_OK || view.byteLength < 2 || view[1] !== wireTags[spec.result]) throw invalid();
  switch (spec.result) {
    case "void":
      return undefined;
    case "boolean":
      if (view.byteLength < 3) throw invalid();
      return view[2] !== 0;
    case "number":
      if (view.byteLength < 10) throw invalid();
      return view.readDoubleLE(2);
    case "string":
    case "bytes":
    case "json": {
      if (view.byteLength < 6) throw invalid();
      const length = view.readUInt32LE(2);
      if (view.byteLength < 6 + length) throw invalid();
      if (spec.result === "string") return view.toString("utf8", 6, 6 + length);
      if (spec.result === "bytes") return new Uint8Array(view.subarray(6, 6 + length));
      return JSON.parse(view.toString("utf8", 6, 6 + length)) as unknown;
    }
  }
}

/** A loaded native module: typed wrappers generated by the CLI call these by function index. */
export interface BoundNativeModule {
  readonly name: string;
  readonly binding: NativeModuleBinding;
  /** Run function `index` on the calling thread. */
  call(index: number, values: readonly unknown[]): unknown;
  /** Run function `index` on the host's native thread pool. */
  callAsync(index: number, values: readonly unknown[]): Promise<unknown>;
}

/**
 * Bind a loaded addon to the signature its generated `index.ts` was produced from.
 *
 * The addon's own manifest is checked against `binding`, so a stale addon (a `main.zig` edited
 * after the last `quickgui modules`) fails at load with a clear message instead of decoding
 * garbage.
 */
export function bindNativeModule(
  name: string,
  addon: unknown,
  binding: NativeModuleBinding,
): BoundNativeModule {
  if (!isNativeModuleAddon(addon)) {
    throw new TypeError(`The native module "${name}" addon does not export manifest, call, and callAsync`);
  }
  if (binding.abi !== NATIVE_MODULE_ABI) {
    throw new Error(
      `The native module "${name}" was generated for ABI ${binding.abi}; this @quickgui/native expects ABI ${NATIVE_MODULE_ABI}. Run \`quickgui modules\`.`,
    );
  }
  const actual = nativeModuleBinding(parseNativeModuleManifest(addon.manifest()));
  if (bindingSignature(actual) !== bindingSignature(binding)) {
    throw new Error(
      `The native module "${name}" was built from a different main.zig than its index.ts. Run \`quickgui modules\` to rebuild it.`,
    );
  }
  const specs = binding.functions;
  const spec = (index: number): NativeFunctionBinding => {
    const entry = specs[index];
    if (!entry) throw new RangeError(`The native module "${name}" has no function ${index}`);
    return entry;
  };
  return {
    name,
    binding,
    call(index, values) {
      const entry = spec(index);
      return decodeNativeResult(entry, addon.call(index, encodeNativeArguments(entry, values, name)), name);
    },
    async callAsync(index, values) {
      const entry = spec(index);
      const encoded = encodeNativeArguments(entry, values, name);
      return decodeNativeResult(entry, await addon.callAsync(index, encoded), name);
    },
  };
}

function isNativeModuleAddon(value: unknown): value is NativeModuleAddon {
  return (
    isRecord(value) &&
    typeof value.manifest === "function" &&
    typeof value.call === "function" &&
    typeof value.callAsync === "function"
  );
}

function isValueSpec(value: unknown): value is NativeValueSpec {
  return (
    isRecord(value) &&
    typeof value.wire === "string" &&
    value.wire in wireTags &&
    isRecord(value.type) &&
    typeof value.type.kind === "string"
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
