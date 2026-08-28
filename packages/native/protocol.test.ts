import { describe, expect, test } from "bun:test";
import {
  MutationBatch,
  NativeNodeTag,
  NO_ANCHOR,
  PropertyCode,
  PROTOCOL_VERSION,
} from "./protocol.ts";

describe("binary mutation protocol", () => {
  test("writes one bounded little-endian batch", () => {
    const batch = new MutationBatch();
    batch.createElement(1, NativeNodeTag.View);
    batch.createText(2, "héllo");
    batch.setProperty(1, PropertyCode.Width, 320);
    batch.insert(1, 2);
    const bytes = batch.finish();
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);

    expect(bytes.subarray(0, 4).toString("utf8")).toBe("QGMB");
    expect(view.getUint16(4, true)).toBe(PROTOCOL_VERSION);
    expect(view.getUint32(6, true)).toBe(4);
    expect(view.getUint32(bytes.byteLength - 4, true)).toBe(NO_ANCHOR);
  });

  test("rejects non-finite numeric properties", () => {
    const batch = new MutationBatch();
    expect(() => batch.setProperty(1, PropertyCode.Width, Number.NaN)).toThrow("finite");
  });

  test("encodes native input, password, and Markdown additions under protocol v3", () => {
    expect(PROTOCOL_VERSION).toBe(3);
    const batch = new MutationBatch();
    batch.createElement(1, NativeNodeTag.Input);
    batch.setProperty(1, PropertyCode.Value, "hello");
    batch.setProperty(1, PropertyCode.Password, true);
    batch.createElement(2, NativeNodeTag.Markdown);
    batch.setProperty(2, PropertyCode.Streaming, true);
    batch.setProperty(2, PropertyCode.ScrollToEndRevision, 3);
    expect(batch.mutationCount).toBe(6);
    expect(batch.finish().byteLength).toBeGreaterThan(10);
  });
});
