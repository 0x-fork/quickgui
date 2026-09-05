import { describe, expect, test } from "bun:test";
import { Buffer } from "node:buffer";

import {
  NATIVE_MODULE_ABI,
  NativeModuleError,
  bindNativeModule,
  decodeNativeResult,
  encodeNativeArguments,
  nativeModuleBinding,
  parseNativeModuleManifest,
  type NativeFunctionBinding,
  type NativeModuleAddon,
  type NativeModuleManifest,
  type NativeWire,
} from "./modules.ts";

const tags: Record<NativeWire, number> = { void: 0, boolean: 1, number: 2, string: 3, bytes: 4, json: 5 };

/** Build a result the way `zig/quickgui.zig` does. */
function okResult(wire: NativeWire, payload?: unknown): Uint8Array {
  const parts: Buffer[] = [Buffer.from([0, tags[wire]])];
  if (wire === "boolean") parts.push(Buffer.from([payload ? 1 : 0]));
  else if (wire === "number") {
    const number = Buffer.alloc(8);
    number.writeDoubleLE(payload as number);
    parts.push(number);
  } else if (wire !== "void") {
    const bytes =
      wire === "json"
        ? Buffer.from(JSON.stringify(payload))
        : Buffer.from(payload as string | Uint8Array);
    const length = Buffer.alloc(4);
    length.writeUInt32LE(bytes.byteLength);
    parts.push(length, bytes);
  }
  return Buffer.concat(parts);
}

function errorResult(name: string): Uint8Array {
  const bytes = Buffer.from(name);
  const length = Buffer.alloc(4);
  length.writeUInt32LE(bytes.byteLength);
  return Buffer.concat([Buffer.from([1]), length, bytes]);
}

const manifest: NativeModuleManifest = {
  abi: NATIVE_MODULE_ABI,
  zig: "0.15.2",
  functions: [
    {
      name: "add",
      params: [
        { wire: "number", type: { kind: "number" } },
        { wire: "number", type: { kind: "number" } },
      ],
      result: { wire: "number", type: { kind: "number" } },
    },
    {
      name: "parse",
      params: [{ injected: "allocator" }, { wire: "string", type: { kind: "string" } }],
      result: {
        wire: "json",
        type: {
          kind: "struct",
          name: "main.Hunk",
          fields: [{ name: "lines", type: { kind: "number" }, hasDefault: false }],
        },
      },
    },
    {
      name: "reverse",
      params: [{ wire: "bytes", type: { kind: "bytes" } }],
      result: { wire: "bytes", type: { kind: "bytes" } },
    },
  ],
};

describe("native module argument encoding", () => {
  test("writes tagged little-endian values in parameter order, skipping injected parameters", () => {
    const spec: NativeFunctionBinding = {
      name: "mixed",
      params: ["allocator", "boolean", "number", "string", "bytes", "json"],
      result: "void",
    };
    const encoded = Buffer.from(
      encodeNativeArguments(spec, [true, 1.5, "hé", new Uint8Array([9, 8]), { a: [1] }]),
    );
    const expected = Buffer.concat([
      Buffer.from([1, 1]),
      (() => {
        const number = Buffer.alloc(9);
        number[0] = 2;
        number.writeDoubleLE(1.5, 1);
        return number;
      })(),
      Buffer.from([3, 3, 0, 0, 0]),
      Buffer.from("hé"),
      Buffer.from([4, 2, 0, 0, 0, 9, 8]),
      Buffer.from([5, 9, 0, 0, 0]),
      Buffer.from('{"a":[1]}'),
    ]);
    expect(encoded.equals(expected)).toBe(true);
  });

  test("accepts bytes for string parameters and rejects wrong types with the argument position", () => {
    const spec: NativeFunctionBinding = { name: "f", params: ["string", "json"], result: "void" };
    const encoded = Buffer.from(encodeNativeArguments(spec, [new Uint8Array([65]), null]));
    expect(encoded.equals(Buffer.concat([Buffer.from([3, 1, 0, 0, 0, 65]), Buffer.from([5, 4, 0, 0, 0]), Buffer.from("null")]))).toBe(true);
    expect(() => encodeNativeArguments(spec, [1, null], "demo")).toThrow(
      "demo.f: argument 1 must be a string or a Uint8Array",
    );
    expect(() => encodeNativeArguments(spec, ["x", undefined], "demo")).toThrow(
      "demo.f: argument 2 must be a JSON-serializable value",
    );
    expect(() => encodeNativeArguments(spec, ["x"], "demo")).toThrow("demo.f expects 2 arguments, got 1");
    expect(() =>
      encodeNativeArguments({ name: "g", params: ["number"], result: "void" }, ["1"], "demo"),
    ).toThrow("demo.g: argument 1 must be a number");
  });
});

describe("native module result decoding", () => {
  const spec = (result: NativeWire): NativeFunctionBinding => ({ name: "f", params: [], result });

  test("decodes every wire kind", () => {
    expect(decodeNativeResult(spec("void"), okResult("void"))).toBeUndefined();
    expect(decodeNativeResult(spec("boolean"), okResult("boolean", true))).toBe(true);
    expect(decodeNativeResult(spec("number"), okResult("number", -2.5))).toBe(-2.5);
    expect(decodeNativeResult(spec("string"), okResult("string", "héllo"))).toBe("héllo");
    const bytes = decodeNativeResult(spec("bytes"), okResult("bytes", new Uint8Array([1, 2])));
    expect(bytes).toBeInstanceOf(Uint8Array);
    expect(Array.from(bytes as Uint8Array)).toEqual([1, 2]);
    expect(decodeNativeResult(spec("json"), okResult("json", { a: [1, null] }))).toEqual({ a: [1, null] });
  });

  test("throws the Zig error name as a NativeModuleError", () => {
    let caught: unknown;
    try {
      decodeNativeResult(spec("number"), errorResult("NotFound"), "git");
    } catch (error) {
      caught = error;
    }
    expect(caught).toBeInstanceOf(NativeModuleError);
    const error = caught as NativeModuleError;
    expect(error.code).toBe("NotFound");
    expect(error.module).toBe("git");
    expect(error.functionName).toBe("f");
    expect(error.message).toBe("git.f failed with NotFound");
  });

  test("rejects truncated or mismatched results instead of reading past them", () => {
    expect(() => decodeNativeResult(spec("number"), new Uint8Array([0, 2, 1]))).toThrow("InvalidResultEncoding");
    expect(() => decodeNativeResult(spec("string"), okResult("number", 1))).toThrow("InvalidResultEncoding");
    expect(() => decodeNativeResult(spec("string"), new Uint8Array([0, 3, 9, 0, 0, 0, 65]))).toThrow(
      "InvalidResultEncoding",
    );
    expect(() => decodeNativeResult(spec("string"), new Uint8Array([1, 9, 0, 0, 0]))).toThrow("InvalidResultEncoding");
  });
});

describe("manifests", () => {
  test("parses the addon manifest and reduces it to the wire binding", () => {
    const parsed = parseNativeModuleManifest(JSON.stringify(manifest));
    expect(parsed).toEqual(manifest);
    expect(nativeModuleBinding(parsed)).toEqual({
      abi: NATIVE_MODULE_ABI,
      functions: [
        { name: "add", params: ["number", "number"], result: "number" },
        { name: "parse", params: ["allocator", "string"], result: "json" },
        { name: "reverse", params: ["bytes"], result: "bytes" },
      ],
    });
  });

  test("rejects other ABI versions and malformed entries", () => {
    expect(() => parseNativeModuleManifest(JSON.stringify({ ...manifest, abi: 99 }))).toThrow("ABI 99");
    expect(() => parseNativeModuleManifest("{")).toThrow("not valid JSON");
    expect(() =>
      parseNativeModuleManifest(JSON.stringify({ abi: 1, functions: [{ name: "x", params: [{}], result: manifest.functions[0]!.result }] })),
    ).toThrow("invalid parameter entry");
  });
});

describe("bindNativeModule", () => {
  function fakeAddon(reported: NativeModuleManifest = manifest): NativeModuleAddon & { calls: Uint8Array[] } {
    const calls: Uint8Array[] = [];
    return {
      calls,
      manifest: () => JSON.stringify(reported),
      call(index, encoded) {
        calls.push(encoded);
        if (index === 0) {
          const view = Buffer.from(encoded);
          return okResult("number", view.readDoubleLE(1) + view.readDoubleLE(10));
        }
        if (index === 1) return okResult("json", { lines: encoded.byteLength - 5 });
        return errorResult("Unsupported");
      },
      async callAsync(index, encoded) {
        return this.call(index, encoded);
      },
    };
  }

  test("routes typed calls through the encoder and decoder, synchronously and asynchronously", async () => {
    const addon = fakeAddon();
    const module = bindNativeModule("demo", addon, nativeModuleBinding(manifest));
    expect(module.call(0, [2, 40])).toBe(42);
    expect(module.call(1, ["abc"])).toEqual({ lines: 3 });
    await expect(module.callAsync(0, [1, 1])).resolves.toBe(2);
    await expect(module.callAsync(2, [new Uint8Array(1)])).rejects.toBeInstanceOf(NativeModuleError);
    expect(() => module.call(7, [])).toThrow("has no function 7");
  });

  test("refuses an addon whose signature differs from the generated binding", () => {
    const stale: NativeModuleManifest = {
      ...manifest,
      functions: [manifest.functions[0]!, { ...manifest.functions[1]!, params: [{ wire: "bytes", type: { kind: "bytes" } }] }],
    };
    expect(() => bindNativeModule("demo", fakeAddon(stale), nativeModuleBinding(manifest))).toThrow(
      'The native module "demo" was built from a different main.zig',
    );
    expect(() => bindNativeModule("demo", {}, nativeModuleBinding(manifest))).toThrow(
      "does not export manifest, call, and callAsync",
    );
  });
});
