# MoonBit syntax parser

`moonbit.wasm` is built from the official [MoonBit Tree-sitter grammar](https://github.com/moonbitlang/tree-sitter-moonbit) at revision `5435c307c6cf2ef0d508a99047b06f35a4308444`, under Apache-2.0 (see LICENSE).

Regenerate with `bun scripts/build-moonbit-parser.ts`. It uses Tree-sitter 0.26.7 and its WASI SDK. The CLI loads this bundled grammar through `web-tree-sitter`; application builds do not compile or download a parser.
