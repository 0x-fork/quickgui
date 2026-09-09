import { app } from "./index.ts";
import * as binding from "./binding.ts";

function validate(name: string, method = "start") {
  if (!/^[a-z][a-z0-9-]{0,63}$/.test(name) || !/^[A-Za-z][A-Za-z0-9_.-]{0,63}$/.test(method))
    throw new TypeError("Invalid native extension name or method");
}
export function invokeExtension<T = unknown>(
  name: string,
  method: string,
  value: unknown = null,
): Promise<T> {
  validate(name, method);
  return binding.invoke(`extension/${name}/${method}`, value);
}
/** One event stream and its requests, owned by the app until closed. */
export class ExtensionSession<T = unknown> {
  readonly ready: Promise<void>;
  readonly #id: number;
  #closed = false;
  readonly #unsubscribe: () => void;
  constructor(
    readonly name: string,
    options: unknown,
    changed: (event: T) => void,
  ) {
    validate(name);
    if (!app.isReady()) throw new Error("Await app.whenReady() before opening an extension");
    const started = binding.startExtension(name, options, (value) => {
      if (!this.#closed) changed(JSON.parse(value));
    });
    this.#id = started.session;
    this.ready = started.ready;
    this.#unsubscribe = app.on("quit", () => {
      this.#closed = true;
    });
    void this.ready.catch(() => {
      this.#closed = true;
      this.#unsubscribe();
    });
  }
  async request<R = unknown>(method: string, value: unknown = null): Promise<R> {
    validate(this.name, method);
    if (method === "start" || method === "stop")
      throw new Error("Use the session constructor and close() for lifecycle");
    await this.ready;
    if (this.#closed) throw new Error("Extension session is closed");
    return invokeExtension(this.name, method, { session: this.#id, value });
  }
  async close(): Promise<void> {
    if (this.#closed) return;
    this.#closed = true;
    this.#unsubscribe();
    try {
      await this.ready;
    } catch {
      return;
    }
    await binding.stopExtension(this.name, this.#id);
  }
}
