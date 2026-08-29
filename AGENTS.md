# Repository guidance

## Architecture

- Implement public framework capabilities in the Rust core whenever possible. JavaScript bindings should adopt and expose the core behavior instead of maintaining a parallel implementation or source of truth; keep logic in JavaScript only when it is inherently specific to that runtime or renderer.
