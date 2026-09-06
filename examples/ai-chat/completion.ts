/** DeepSeek's streaming Chat Completions protocol using the native runtime's fetch. */
export interface ModelMessage { role: "user" | "assistant" | "system"; content: string; }
interface CompletionChunk {
  choices?: { delta?: { content?: string | null }; finish_reason?: string | null }[];
  error?: { message?: string };
}

const MAX_EVENT_BYTES = 1024 * 1024;

/** Byte framing keeps UTF-8 characters intact even when the network splits a code point. */
export class CompletionStream {
  #pending = new Uint8Array(0);
  #data = "";
  #done = false;
  #onText: (text: string) => boolean;
  constructor(onText: (text: string) => boolean) { this.#onText = onText; }
  get done(): boolean { return this.#done; }

  push(chunk: Uint8Array): void {
    if (this.#done) return;
    const bytes = new Uint8Array(this.#pending.byteLength + chunk.byteLength);
    bytes.set(this.#pending, 0);
    bytes.set(chunk, this.#pending.byteLength);
    let start = 0;
    for (let index = 0; index < bytes.byteLength; index += 1) {
      if (bytes[index] !== 10) continue;
      if (index - start > MAX_EVENT_BYTES) throw new Error("DeepSeek sent an oversized stream event");
      const line = new TextDecoder().decode(bytes.subarray(start, index)).replace(/\r$/, "");
      start = index + 1;
      this.#line(line);
      if (this.#done) break;
    }
    this.#pending = this.#done ? new Uint8Array(0) : bytes.slice(start);
    if (this.#pending.byteLength > MAX_EVENT_BYTES) throw new Error("DeepSeek sent an oversized stream event");
  }

  finish(): void {
    if (this.#done) return;
    if (this.#pending.byteLength > 0) this.#line(new TextDecoder().decode(this.#pending).replace(/\r$/, ""));
    this.#line("");
    if (!this.#done) throw new Error("DeepSeek closed the stream before completing the response");
  }

  #line(line: string): void {
    if (line === "") {
      const data = this.#data.trim();
      this.#data = "";
      if (!data) return;
      if (data === "[DONE]") { this.#done = true; return; }
      const event: CompletionChunk | null = JSON.parse(data);
      if (event === null) throw new Error("DeepSeek sent an invalid stream event");
      if (event.error !== undefined) throw new Error(event.error.message ?? "DeepSeek request failed");
      const choices = event.choices;
      if (choices !== undefined && choices.length > 0) {
        const content = choices[0]!.delta?.content;
        if (content !== undefined && content !== null && content !== "" && !this.#onText(content)) this.#done = true;
      }
    } else if (line.startsWith("data:")) {
      this.#data += (this.#data ? "\n" : "") + line.slice(5).replace(/^ /, "");
      if (this.#data.length > MAX_EVENT_BYTES) throw new Error("DeepSeek sent an oversized stream event");
    }
  }
}

export async function streamCompletion(
  apiKey: string,
  messages: ModelMessage[],
  signal: AbortSignal,
  onText: (text: string) => boolean,
): Promise<void> {
  const response = await fetch("https://api.deepseek.com/chat/completions", {
    method: "POST",
    headers: { "Content-Type": "application/json", Authorization: `Bearer ${apiKey}` },
    body: JSON.stringify({ model: "deepseek-v4-flash", thinking: { type: "disabled" }, messages, stream: true }),
    signal,
  });
  if (!response.ok) throw new Error(`DeepSeek request failed (HTTP ${response.status})`);
  const body = response.body;
  if (body === null) throw new Error("DeepSeek returned no response stream");
  const reader = body.getReader();
  const parser = new CompletionStream(onText);
  try {
    while (!parser.done) {
      const result = await reader.read();
      if (result.done) { parser.finish(); break; }
      parser.push(result.value);
    }
  } finally {
    await reader.cancel();
  }
}
