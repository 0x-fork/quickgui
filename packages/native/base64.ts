/** Base64 transport for byte payloads nested inside JSON declarations and results. */

const ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

function alphabetIndex(code: number): number {
  if (code >= 65 && code <= 90) return code - 65;
  if (code >= 97 && code <= 122) return code - 71;
  if (code >= 48 && code <= 57) return code + 4;
  if (code === 43 || code === 45) return 62;
  if (code === 47 || code === 95) return 63;
  return -1;
}

export function encodeBase64(bytes: Uint8Array): string {
  const parts: string[] = [];
  let i = 0;
  while (i + 2 < bytes.length) {
    const a = bytes[i]!;
    const b = bytes[i + 1]!;
    const c = bytes[i + 2]!;
    parts.push(ALPHABET.charAt(a >> 2));
    parts.push(ALPHABET.charAt(((a & 3) << 4) | (b >> 4)));
    parts.push(ALPHABET.charAt(((b & 15) << 2) | (c >> 6)));
    parts.push(ALPHABET.charAt(c & 63));
    i += 3;
  }
  const remaining = bytes.length - i;
  if (remaining === 1) {
    const a = bytes[i]!;
    parts.push(ALPHABET.charAt(a >> 2));
    parts.push(ALPHABET.charAt((a & 3) << 4));
    parts.push("==");
  } else if (remaining === 2) {
    const a = bytes[i]!;
    const b = bytes[i + 1]!;
    parts.push(ALPHABET.charAt(a >> 2));
    parts.push(ALPHABET.charAt(((a & 3) << 4) | (b >> 4)));
    parts.push(ALPHABET.charAt((b & 15) << 2));
    parts.push("=");
  }
  return parts.join("");
}

export function decodeBase64(text: string): Uint8Array {
  let length = text.length;
  while (length > 0 && text.charCodeAt(length - 1) === 61) length -= 1;
  const output = new Uint8Array(Math.floor((length * 3) / 4));
  let outputIndex = 0;
  let buffer = 0;
  let bits = 0;
  for (let i = 0; i < length; i++) {
    const value = alphabetIndex(text.charCodeAt(i));
    if (value < 0) continue;
    buffer = (buffer << 6) | value;
    bits += 6;
    if (bits >= 8) {
      bits -= 8;
      if (outputIndex < output.length) {
        output[outputIndex] = (buffer >> bits) & 255;
        outputIndex += 1;
      }
    }
  }
  return outputIndex === output.length ? output : output.subarray(0, outputIndex);
}
