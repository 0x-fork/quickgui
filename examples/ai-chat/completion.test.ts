import { expect, test } from "bun:test";
import { CompletionStream } from "./completion.ts";
const encode = (text: string) => new TextEncoder().encode(text);

test("completion SSE handles UTF-8 split at every byte and CRLF", () => {
  let text = "";
  const parser = new CompletionStream((chunk) => { text += chunk; return true; });
  const bytes = encode(': keepalive\r\ndata: {"choices":[{"delta":{"content":"Hello 🙂 世界"}}]}\r\n\r\ndata: [DONE]\r\n\r\n');
  for (const byte of bytes) parser.push(new Uint8Array([byte]));
  parser.finish();
  expect(text).toBe("Hello 🙂 世界");
  expect(parser.done).toBe(true);
});

test("completion SSE ignores role-only chunks and stops when consumer reaches its bound", () => {
  let count = 0;
  const parser = new CompletionStream(() => { count += 1; return false; });
  parser.push(encode('data: {"choices":[{"delta":{"role":"assistant","content":null}}]}\n\ndata: {"choices":[{"delta":{"content":"a"}}]}\n\ndata: {"choices":[{"delta":{"content":"b"}}]}\n\n'));
  expect(count).toBe(1);
  expect(parser.done).toBe(true);
});

test("completion SSE reports provider errors, truncated streams and oversized frames", () => {
  const parser = new CompletionStream(() => true);
  expect(() => parser.push(encode('data: {"error":{"message":"Unavailable"}}\n\n'))).toThrow("Unavailable");
  expect(() => new CompletionStream(() => true).finish()).toThrow("before completing");
  expect(() => new CompletionStream(() => true).push(encode("x".repeat(1024 * 1024 + 1)))).toThrow("oversized");
});
