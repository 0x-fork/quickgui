# ai-chat

A DeepSeek chat client with streaming Markdown, cancellation, conversation persistence, and a Keychain-backed API key. Set DEEPSEEK_API_KEY or use Provider settings. Live completion requires your own provider access.

The application is Go. Pass components directly through `native.WindowOptions.Component`; signals update individual retained nodes. The Rust shared library is loaded in process using purego.

From the repository root:

```console
bun install
bun run build:native # once
bun packages/cli/src/cli.ts dev --project examples/ai-chat
```

Check the application without CGO:

```console
CGO_ENABLED=0 go -C examples/ai-chat test ./...
```

See [main.go](main.go) and the [Go guide](../../docs/go.md). macOS packaging uses Xcode Command Line Tools; Go 1.23+ and Bun are required.
