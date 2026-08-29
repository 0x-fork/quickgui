# QuickGUI Solid AI chat

This example streams `deepseek-chat` with the Vercel AI SDK and renders both user and assistant
messages through QuickGUI core's retained native Markdown component.

```console
cd examples/ai-chat-solid
bun run dev
```

Set `DEEPSEEK_API_KEY` before launching, or enter the key in the `SystemPopover` Provider settings.
The field is masked by default and has an explicit Reveal/Hide control. On macOS, keys entered in the
UI are stored in Keychain and never written into chat data. The sidebar keeps up to 100 conversations,
restores the active conversation, and keeps a separate draft for every chat in the platform's local
application data directory. Each transcript retains at most 101 messages, model context is capped at
96,000 characters, and each response is capped at 256,000 characters. Stream commits are coalesced
to at most one native update every 24 ms.
