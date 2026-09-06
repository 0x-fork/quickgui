# QuickGUI AI chat

This example streams `deepseek-v4-flash` through the native runtime's `fetch` and renders user and
assistant messages with QuickGUI core's retained Markdown component. The SSE parser keeps split
UTF-8 characters intact and supports cancellation. The app compiles with scriptc.

```console
cd examples/ai-chat
bun run dev
```

Set `DEEPSEEK_API_KEY` before launching, or enter the key in the `SystemPopover` Provider settings.
The field is masked by default and has an explicit Reveal/Hide control. On macOS, keys entered in the
UI are stored in Keychain and never written into chat data. The sidebar keeps up to 100 conversations,
restores the active conversation, and keeps a separate draft for every chat in the platform's local
application data directory. Each transcript retains at most 101 messages, model context is capped at
96,000 characters, and each response is capped at 256,000 characters. Stream commits are coalesced
to at most one native update every 24 ms.
