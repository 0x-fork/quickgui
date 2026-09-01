# Repository guidance

## Architecture

- Implement public framework capabilities in the Rust core whenever possible. JavaScript bindings should adopt and expose the core behavior instead of maintaining a parallel implementation or source of truth; keep logic in JavaScript only when it is inherently specific to that runtime or renderer.
- The hosted JavaScript/native boundary must never synchronously wait for native main-thread execution. Enqueue fire-and-forget mutations, allocate constructor handles locally, and expose every result, lifecycle outcome, or native request acceptance through a Promise-backed asynchronous task.
