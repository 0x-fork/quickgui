import { createDeepSeek } from "@ai-sdk/deepseek";
import {
  App,
  Button,
  Input,
  Markdown,
  Text,
  View,
  Window,
  type NativeNode,
  render,
} from "@quickgui/solid";
import { streamText } from "ai";
import { For, createSignal, flush } from "solid-js";
import {
  deleteDeepSeekApiKey,
  loadDeepSeekApiKey,
  saveDeepSeekApiKey,
} from "./credentials.ts";
import {
  type ChatMessage,
  type Conversation,
  type ConversationHistory,
  conversationHistoryPath,
  createConversation,
  loadConversationHistory,
  saveConversationHistory,
  titleFromPrompt,
} from "./history.ts";

const STREAM_COMMIT_INTERVAL_MS = 24;
const MAX_TRANSCRIPT_MESSAGES = 101;
const MAX_CONTEXT_CHARACTERS = 96_000;
const MAX_PROMPT_CHARACTERS = 16_000;
// Fits below the native bridge's 1 MiB string bound even when every code unit uses 3 UTF-8 bytes.
const MAX_RESPONSE_CHARACTERS = 256_000;
const MAX_CONVERSATIONS = 100;
const HISTORY_WRITE_DELAY_MS = 350;

interface ActiveChatRequest {
  conversationId: number;
  controller: AbortController;
}

const historyPath = conversationHistoryPath();
const restoredHistory = await loadConversationHistory(historyPath);
const firstConversation = createConversation(1, 1);
const initialHistory: ConversationHistory = restoredHistory ?? {
  activeConversationId: firstConversation.id,
  conversations: [firstConversation],
};
const environmentApiKey = process.env.DEEPSEEK_API_KEY?.trim() ?? "";
let storedApiKey = "";
if (!environmentApiKey) {
  try {
    storedApiKey = (await loadDeepSeekApiKey()).trim();
  } catch (error) {
    console.error("Unable to load the DeepSeek API key from macOS Keychain", error);
  }
}
const app = new App();
const mainWindow = new Window({
  title: "QuickGUI AI Chat",
  width: 1080,
  height: 720,
  minimumWidth: 760,
  minimumHeight: 480,
  background: "#0b0d12",
  titleBarStyle: "hiddenInset",
  trafficLightPosition: { x: 16, y: 15 },
});

const [apiKey, setApiKey] = createSignal(environmentApiKey || storedApiKey);
const [draft, setDraft] = createSignal(
  initialHistory.conversations.find(
    (conversation) => conversation.id === initialHistory.activeConversationId,
  )?.draft ?? "",
);
const [conversations, setConversations] = createSignal(initialHistory.conversations);
const [activeConversationId, setActiveConversationId] = createSignal(
  initialHistory.activeConversationId,
);
const [activeRequest, setActiveRequest] = createSignal<ActiveChatRequest>();
const [scrollRevision, setScrollRevision] = createSignal(0);
let nextConversationId =
  Math.max(...initialHistory.conversations.map((conversation) => conversation.id)) + 1;
let nextMessageId =
  Math.max(
    ...initialHistory.conversations.flatMap((conversation) =>
      conversation.messages.map((message) => message.id),
    ),
  ) + 1;
let settingsWindow: Window | undefined;
let providerSettingsButton: NativeNode | undefined;
let composer: NativeNode | undefined;
let historyWriteTimer: ReturnType<typeof setTimeout> | undefined;
let pendingHistory: ConversationHistory | undefined;
let historyWrite = Promise.resolve();

function activeConversation(): Conversation {
  return (
    conversations().find((conversation) => conversation.id === activeConversationId()) ??
    conversations()[0]!
  );
}

function messages(): ChatMessage[] {
  return activeConversation().messages;
}

function commitConversations(update: (current: Conversation[]) => Conversation[]): void {
  setConversations(update);
  scheduleHistoryPersist();
}

function scheduleHistoryPersist(): void {
  if (historyWriteTimer) clearTimeout(historyWriteTimer);
  historyWriteTimer = setTimeout(() => {
    historyWriteTimer = undefined;
    pendingHistory = historySnapshot();
    queueHistoryWrite();
  }, HISTORY_WRITE_DELAY_MS);
}

function historySnapshot(): ConversationHistory {
  const currentConversationId = activeConversationId();
  const currentDraft = draft();
  return {
    activeConversationId: currentConversationId,
    conversations: conversations().map((conversation) =>
      conversation.id === currentConversationId && conversation.draft !== currentDraft
        ? { ...conversation, draft: currentDraft }
        : conversation,
    ),
  };
}

function retainActiveDraft(): void {
  const currentConversationId = activeConversationId();
  const currentDraft = draft();
  setConversations((current) =>
    current.map((conversation) =>
      conversation.id === currentConversationId && conversation.draft !== currentDraft
        ? { ...conversation, draft: currentDraft }
        : conversation,
    ),
  );
}

function queueHistoryWrite(): void {
  const snapshot = pendingHistory;
  pendingHistory = undefined;
  if (!snapshot) return;
  historyWrite = historyWrite
    .catch(() => {})
    .then(() => saveConversationHistory(snapshot, historyPath))
    .catch((error) => console.error("Unable to save chat history", error));
}

async function flushHistoryPersist(): Promise<void> {
  if (historyWriteTimer) clearTimeout(historyWriteTimer);
  historyWriteTimer = undefined;
  pendingHistory = historySnapshot();
  queueHistoryWrite();
  await historyWrite;
}

function updateMessage(
  conversationId: number,
  messageId: number,
  update: Partial<ChatMessage>,
): void {
  commitConversations((current) =>
    current.map((conversation) =>
      conversation.id === conversationId
        ? {
            ...conversation,
            messages: conversation.messages.map((message) =>
              message.id === messageId ? { ...message, ...update } : message,
            ),
            updatedAt: Date.now(),
          }
        : conversation,
    ),
  );
  if (activeConversationId() === conversationId) {
    setScrollRevision((revision) => revision + 1);
  }
  flush();
}

function appendTranscript(conversationId: number, ...incoming: ChatMessage[]): void {
  commitConversations((current) => {
    const updated = current.map((conversation) => {
      if (conversation.id !== conversationId) return conversation;
      const welcome = conversation.messages[0];
      const history = conversation.messages.slice(1);
      const retainedCount = Math.max(
        0,
        MAX_TRANSCRIPT_MESSAGES - incoming.length - (welcome ? 1 : 0),
      );
      const retained = retainedCount === 0 ? [] : history.slice(-retainedCount);
      return {
        ...conversation,
        title:
          conversation.title === "New chat" && incoming[0]?.role === "user"
            ? titleFromPrompt(incoming[0].content)
            : conversation.title,
        messages: [...(welcome ? [welcome] : []), ...retained, ...incoming],
        updatedAt: Date.now(),
      };
    });
    return updated.sort((left, right) => right.updatedAt - left.updatedAt);
  });
}

function selectConversation(id: number): void {
  if (activeRequest() || id === activeConversationId()) return;
  const nextConversation = conversations().find((conversation) => conversation.id === id);
  if (!nextConversation) return;
  retainActiveDraft();
  setActiveConversationId(id);
  setDraft(nextConversation.draft);
  setScrollRevision((revision) => revision + 1);
  scheduleHistoryPersist();
  flush();
}

function startNewConversation(): void {
  if (activeRequest()) return;
  const current = activeConversation();
  if (current.title === "New chat" && current.messages.length === 1 && !draft()) {
    return;
  }
  retainActiveDraft();
  const conversation = createConversation(nextConversationId++, nextMessageId++);
  setActiveConversationId(conversation.id);
  commitConversations((currentConversations) =>
    [conversation, ...currentConversations].slice(0, MAX_CONVERSATIONS),
  );
  setDraft("");
  setScrollRevision((revision) => revision + 1);
  scheduleHistoryPersist();
  flush();
}

function modelConversation(current: ChatMessage[], userMessage: ChatMessage) {
  const candidates = [...current.slice(1), userMessage].filter(
    (message) => !message.failed && !message.streaming,
  );
  const retained: ChatMessage[] = [];
  let characters = 0;
  for (let index = candidates.length - 1; index >= 0; index--) {
    const message = candidates[index]!;
    if (retained.length > 0 && characters + message.content.length > MAX_CONTEXT_CHARACTERS) break;
    retained.push(message);
    characters += message.content.length;
  }
  return retained.reverse().map((message) => ({
    role: message.role,
    content: message.content,
  }));
}

function createStreamingCommitter(conversationId: number, messageId: number) {
  let accumulated = "";
  let timer: ReturnType<typeof setTimeout> | undefined;
  let lastCommit = 0;

  const commit = () => {
    timer = undefined;
    lastCommit = performance.now();
    updateMessage(conversationId, messageId, { content: accumulated });
  };

  return {
    append(chunk: string) {
      const remaining = MAX_RESPONSE_CHARACTERS - accumulated.length;
      if (remaining <= 0) return false;
      let accepted = chunk;
      if (chunk.length > remaining) {
        let end = remaining;
        const last = chunk.charCodeAt(end - 1);
        const next = chunk.charCodeAt(end);
        if (last >= 0xd800 && last <= 0xdbff && next >= 0xdc00 && next <= 0xdfff) end--;
        accepted = chunk.slice(0, end);
      }
      accumulated += accepted;
      const complete = accepted.length === chunk.length;
      if (timer) return complete;
      const delay = Math.max(0, STREAM_COMMIT_INTERVAL_MS - (performance.now() - lastCommit));
      timer = setTimeout(commit, delay);
      return complete;
    },
    finish() {
      if (timer) clearTimeout(timer);
      timer = undefined;
      return accumulated;
    },
  };
}

async function sendMessage(submittedValue?: string): Promise<void> {
  const prompt = (submittedValue ?? draft()).slice(0, MAX_PROMPT_CHARACTERS).trim();
  if (!prompt || activeRequest()) return;
  const key = apiKey();
  if (!key) {
    openProviderSettings();
    return;
  }

  const conversation = activeConversation();
  const conversationId = conversation.id;
  const userMessage: ChatMessage = {
    id: nextMessageId++,
    role: "user",
    content: prompt,
  };
  const assistantMessage: ChatMessage = {
    id: nextMessageId++,
    role: "assistant",
    content: "",
    streaming: true,
  };
  const modelMessages = modelConversation(conversation.messages, userMessage);

  setDraft("");
  appendTranscript(conversationId, userMessage, assistantMessage);
  setScrollRevision((revision) => revision + 1);
  const controller = new AbortController();
  setActiveRequest({ conversationId, controller });
  // Paint the submitted message and streaming placeholder before waiting on the provider.
  flush();
  const committer = createStreamingCommitter(conversationId, assistantMessage.id);
  let responseLimitReached = false;

  try {
    const deepseek = createDeepSeek({ apiKey: key });
    const result = streamText({
      model: deepseek("deepseek-chat"),
      messages: modelMessages,
      abortSignal: controller.signal,
    });
    for await (const chunk of result.textStream) {
      if (!committer.append(chunk)) {
        responseLimitReached = true;
        controller.abort();
        break;
      }
    }
    const content = committer.finish();
    updateMessage(conversationId, assistantMessage.id, {
      content: responseLimitReached
        ? `${content}\n\n_Response stopped at the local 256,000-character safety limit._`
        : content || "_DeepSeek returned an empty response._",
      streaming: false,
    });
  } catch (error) {
    const partial = committer.finish();
    if (responseLimitReached) {
      updateMessage(conversationId, assistantMessage.id, {
        content: `${partial}\n\n_Response stopped at the local 256,000-character safety limit._`,
        streaming: false,
      });
    } else if (controller.signal.aborted) {
      updateMessage(conversationId, assistantMessage.id, {
        content: partial || "_Generation stopped._",
        streaming: false,
      });
    } else {
      const detail = error instanceof Error ? error.message : String(error);
      updateMessage(conversationId, assistantMessage.id, {
        content: `${partial}${partial ? "\n\n" : ""}> **Request failed:** ${detail}`,
        streaming: false,
        failed: true,
      });
    }
  } finally {
    if (activeRequest()?.controller === controller) {
      setActiveRequest(undefined);
      flush();
    }
  }
}

function openProviderSettings(anchor = providerSettingsButton): void {
  if (settingsWindow && !settingsWindow.closed) return;
  if (!anchor) return;
  const window = new Window({
    title: "DeepSeek settings",
    anchor,
    width: 460,
    height: 310,
    placement: "bottom-end",
    gap: 8,
  });
  settingsWindow = window;
  window.onClose(() => {
    if (settingsWindow === window) settingsWindow = undefined;
  });
  render(() => <ProviderSettings window={window} />, window);
}

function ProviderSettings(props: { window: Window }) {
  const [value, setValue] = createSignal(apiKey());
  const [revealed, setRevealed] = createSignal(false);
  const [errorMessage, setErrorMessage] = createSignal("");
  const [saving, setSaving] = createSignal(false);
  let keyInput: NativeNode | undefined;

  const save = async (submittedValue?: string) => {
    const nextValue = (submittedValue ?? value()).trim();
    if (!nextValue || saving()) return;
    setSaving(true);
    setErrorMessage("");
    try {
      await saveDeepSeekApiKey(nextValue);
      setApiKey(nextValue);
      flush();
      props.window.close();
    } catch (error) {
      setSaving(false);
      setErrorMessage(error instanceof Error ? error.message : String(error));
      flush();
    }
  };

  const remove = async () => {
    if (saving()) return;
    setSaving(true);
    setErrorMessage("");
    try {
      await deleteDeepSeekApiKey();
      setApiKey("");
      flush();
      props.window.close();
    } catch (error) {
      setSaving(false);
      setErrorMessage(error instanceof Error ? error.message : String(error));
      flush();
    }
  };

  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        width: "100%",
        height: "100%",
        gap: 14,
        padding: 20,
        backgroundColor: "#11141b",
        color: "#f4f4f5",
        borderWidth: 1,
        borderColor: "#343843",
        borderRadius: 12,
      }}
    >
      <Text style={{ fontSize: 22, lineHeight: 28, fontWeight: 700 }}>DeepSeek provider</Text>
      <Text style={{ color: "#a1a1aa", fontSize: 13, lineHeight: 19 }}>
        Enter DEEPSEEK_API_KEY. QuickGUI stores it securely in your macOS Keychain, separate from
        chat history.
      </Text>
      <View
        style={{
          display: "flex",
          flexDirection: "row",
          width: "100%",
          gap: 8,
        }}
      >
        <Input
          ref={(node) => {
            keyInput = node;
          }}
          type={revealed() ? "text" : "password"}
          value={value()}
          placeholder="sk-..."
          aria-label="DeepSeek API key"
          onInput={(event) => setValue((event.value ?? "").slice(0, 2_048))}
          onSubmit={(event) => void save(event.value)}
          style={{
            display: "flex",
            flex: 1,
            minWidth: 0,
            height: 42,
            paddingLeft: 12,
            paddingRight: 12,
            backgroundColor: "#090b10",
            color: "#fafafa",
            borderWidth: 1,
            borderColor: "#343843",
            borderRadius: 8,
          }}
        />
        <Button
          aria-label={revealed() ? "Hide API key" : "Reveal API key"}
          onClick={() => {
            setRevealed((current) => !current);
            flush();
            keyInput?.focus();
          }}
          disabled={saving()}
          style={{ ...secondaryButtonStyle, width: 76, flexShrink: 0 }}
        >
          {revealed() ? "Hide" : "Reveal"}
        </Button>
      </View>
      {errorMessage() ? (
        <Text style={{ color: "#fca5a5", fontSize: 12, lineHeight: 17 }}>{errorMessage()}</Text>
      ) : null}
      <View
        style={{
          display: "flex",
          flexDirection: "row",
          justifyContent: "flex-end",
          gap: 10,
        }}
      >
        {apiKey() ? (
          <Button
            onClick={() => void remove()}
            disabled={saving()}
            style={{ ...secondaryButtonStyle, color: "#fca5a5" }}
          >
            Remove key
          </Button>
        ) : null}
        <Button
          onClick={() => props.window.close()}
          disabled={saving()}
          style={secondaryButtonStyle}
        >
          Cancel
        </Button>
        <Button
          onClick={() => void save()}
          disabled={saving() || !value().trim()}
          style={primaryButtonStyle}
        >
          {saving() ? "Saving…" : "Save"}
        </Button>
      </View>
    </View>
  );
}

function MessageCard(props: { message: ChatMessage }) {
  const assistant = () => props.message.role === "assistant";
  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        width: "100%",
        maxWidth: 760,
        gap: 8,
        padding: 16,
        backgroundColor: assistant() ? "#141821" : "#15243a",
        borderWidth: 1,
        borderColor: props.message.failed ? "#7f1d1d" : assistant() ? "#282d38" : "#24466f",
        borderRadius: 12,
      }}
    >
      <Text style={{ color: assistant() ? "#a1a1aa" : "#93c5fd", fontSize: 12, fontWeight: 700 }}>
        {assistant() ? "DEEPSEEK" : "YOU"}
      </Text>
      <Markdown
        content={props.message.content || "▍"}
        streaming={props.message.streaming ?? false}
        style={{
          width: "100%",
          minWidth: 0,
          color: "#e4e4e7",
          fontSize: 15,
          lineHeight: 23,
          markdownMutedColor: "#a1a1aa",
          markdownLinkColor: "#60a5fa",
          markdownCodeTextColor: "#e2e8f0",
          markdownCodeBackground: "#090b10",
          markdownBorderColor: "#343843",
          markdownBlockGap: 10,
          markdownCodeFontSize: 13,
        }}
      />
    </View>
  );
}

function historyTimestamp(timestamp: number): string {
  const date = new Date(timestamp);
  const today = new Date();
  if (date.toDateString() === today.toDateString()) {
    return date.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  }
  return date.toLocaleDateString([], { month: "short", day: "numeric" });
}

function HistorySidebar() {
  const busy = () => activeRequest() !== undefined;

  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        width: 244,
        height: "100%",
        minHeight: 0,
        flexShrink: 0,
        backgroundColor: "#0e1118",
        color: "#f4f4f5",
      }}
    >
      <View
        style={{
          display: "flex",
          flexDirection: "row",
          height: 54,
          flexShrink: 0,
          alignItems: "center",
          paddingLeft: 86,
          paddingRight: 12,
          appRegion: "drag",
        }}
      >
        <Text style={{ fontSize: 13, fontWeight: 700 }}>Chats</Text>
      </View>

      <Button
        aria-label="New chat"
        disabled={busy()}
        onClick={startNewConversation}
        style={{
          ...secondaryButtonStyle,
          height: 38,
          marginRight: 12,
          marginLeft: 12,
          paddingRight: 11,
          paddingLeft: 11,
          backgroundColor: "#1d2330",
          color: "#f4f4f5",
          opacity: busy() ? 0.55 : 1,
        }}
      >
        <View
          style={{
            display: "flex",
            flexDirection: "row",
            width: "100%",
            height: "100%",
            alignItems: "center",
            justifyContent: "flex-start",
            gap: 8,
          }}
        >
          <Text style={{ color: "#a1a1aa", fontSize: 17, lineHeight: 20 }}>+</Text>
          <Text style={{ fontSize: 13, fontWeight: 600 }}>New chat</Text>
        </View>
      </Button>

      <Text
        style={{
          marginTop: 18,
          marginRight: 14,
          marginBottom: 7,
          marginLeft: 14,
          color: "#71717a",
          fontSize: 11,
          fontWeight: 700,
        }}
      >
        RECENT
      </Text>

      <View
        style={{
          display: "flex",
          flexDirection: "column",
          flex: 1,
          minHeight: 0,
          gap: 4,
          overflowY: "auto",
          paddingRight: 8,
          paddingBottom: 8,
          paddingLeft: 8,
        }}
      >
        <For each={conversations()} keyed={(conversation) => conversation.id}>
          {(conversation) => {
            const selected = () => conversation().id === activeConversationId();
            return (
              <Button
                aria-label={`Open chat ${conversation().title}`}
                disabled={busy()}
                onClick={() => selectConversation(conversation().id)}
                style={{
                  display: "flex",
                  flexDirection: "column",
                  width: "100%",
                  height: 57,
                  flexShrink: 0,
                  alignItems: "flex-start",
                  justifyContent: "center",
                  gap: 4,
                  paddingRight: 10,
                  paddingLeft: 10,
                  backgroundColor: selected() ? "#242a37" : "#0e1118",
                  color: selected() ? "#fafafa" : "#d4d4d8",
                  borderRadius: 8,
                  cursor: "default",
                  userSelect: "none",
                  opacity: busy() && !selected() ? 0.55 : 1,
                }}
              >
                <Text
                  style={{
                    width: "100%",
                    minWidth: 0,
                    fontSize: 13,
                    fontWeight: selected() ? 700 : 500,
                    lineClamp: 1,
                    whiteSpace: "nowrap",
                    textOverflow: "ellipsis",
                  }}
                >
                  {conversation().title}
                </Text>
                <Text style={{ color: "#71717a", fontSize: 10 }}>
                  {historyTimestamp(conversation().updatedAt)}
                </Text>
              </Button>
            );
          }}
        </For>
      </View>

      <Text
        style={{
          flexShrink: 0,
          paddingTop: 10,
          paddingRight: 14,
          paddingBottom: 13,
          paddingLeft: 14,
          color: "#52525b",
          fontSize: 10,
        }}
      >
        History is stored locally
      </Text>
    </View>
  );
}

function Chat() {
  const busy = () => activeRequest() !== undefined;
  const canSend = () => draft().trim().length > 0 && !busy();

  return (
    <View
      style={{
        display: "flex",
        flexDirection: "row",
        width: "100%",
        height: "100%",
        minWidth: 0,
        minHeight: 0,
        backgroundColor: "#0b0d12",
        color: "#f4f4f5",
      }}
    >
      <HistorySidebar />
      <View style={{ width: 1, height: "100%", flexShrink: 0, backgroundColor: "#22262f" }} />
      <View
        style={{
          display: "flex",
          flexDirection: "column",
          flex: 1,
          width: "100%",
          height: "100%",
          minWidth: 0,
          minHeight: 0,
        }}
      >
        <View
          style={{
            display: "flex",
            flexDirection: "row",
            height: 54,
            flexShrink: 0,
            alignItems: "center",
            justifyContent: "space-between",
            paddingLeft: 18,
            paddingRight: 14,
            appRegion: "drag",
            borderWidth: 1,
            borderColor: "#22262f",
          }}
        >
          <View style={{ display: "flex", flexDirection: "column", minWidth: 0, gap: 2 }}>
            <Text
              style={{
                maxWidth: 430,
                fontWeight: 700,
                lineClamp: 1,
                whiteSpace: "nowrap",
                textOverflow: "ellipsis",
              }}
            >
              {activeConversation().title}
            </Text>
            <Text style={{ color: "#71717a", fontSize: 11 }}>
              DeepSeek · native streaming Markdown
            </Text>
          </View>
          <Button
            ref={(node) => {
              providerSettingsButton = node;
            }}
            onClick={(event) => openProviderSettings(event.currentTarget)}
            style={{ ...secondaryButtonStyle, appRegion: "no-drag" }}
          >
            {apiKey() ? "Provider settings" : "Set API key"}
          </Button>
        </View>

        <View
          style={{
            display: "flex",
            flex: 1,
            minHeight: 0,
            overflowY: "auto",
            scrollToEndRevision: scrollRevision(),
            alignItems: "center",
            paddingTop: 24,
            paddingRight: 28,
            paddingBottom: 24,
            paddingLeft: 28,
          }}
        >
          <View
            style={{
              display: "flex",
              flexDirection: "column",
              width: "100%",
              maxWidth: 760,
              gap: 14,
            }}
          >
            <For each={messages()} keyed={(message) => message.id}>
              {(message) => <MessageCard message={message()} />}
            </For>
          </View>
        </View>

        <View
          style={{
            display: "flex",
            flexDirection: "column",
            flexShrink: 0,
            gap: 9,
            paddingTop: 12,
            paddingRight: 28,
            paddingBottom: 18,
            paddingLeft: 28,
            borderWidth: 1,
            borderColor: "#22262f",
          }}
        >
          <View
            style={{
              display: "flex",
              flexDirection: "row",
              width: "100%",
              gap: 10,
              maxWidth: 760,
              alignSelf: "center",
            }}
          >
            <Input
              ref={(node) => {
                composer = node;
              }}
              value={draft()}
              placeholder="Message DeepSeek…"
              aria-label="Message"
              onInput={(event) => {
                setDraft((event.value ?? "").slice(0, MAX_PROMPT_CHARACTERS));
                scheduleHistoryPersist();
              }}
              onSubmit={(event) => void sendMessage(event.value)}
              style={{
                display: "flex",
                flex: 1,
                minWidth: 0,
                height: 44,
                paddingLeft: 13,
                paddingRight: 13,
                backgroundColor: "#141821",
                color: "#f4f4f5",
                borderWidth: 1,
                borderColor: "#343843",
                borderRadius: 9,
              }}
            />
            <Button
              onClick={() => {
                if (busy()) activeRequest()?.controller.abort();
                else void sendMessage();
                composer?.focus();
              }}
              disabled={!busy() && !canSend()}
              style={busy() ? stopButtonStyle : primaryButtonStyle}
            >
              {busy() ? "Stop" : "Send"}
            </Button>
          </View>
          <Text
            style={{
              alignSelf: "center",
              width: "100%",
              maxWidth: 760,
              color: "#71717a",
              fontSize: 11,
            }}
          >
            {busy()
              ? "Streaming · Stop preserves the partial response"
              : apiKey()
                ? "Return to send · API key is in Keychain · chats and drafts are stored locally"
                : "Drafts are stored locally · sending opens Provider settings"}
          </Text>
        </View>
      </View>
    </View>
  );
}

const secondaryButtonStyle = {
  display: "flex" as const,
  height: 36,
  paddingLeft: 14,
  paddingRight: 14,
  alignItems: "center" as const,
  justifyContent: "center" as const,
  backgroundColor: "#242833",
  color: "#e4e4e7",
  borderRadius: 8,
  cursor: "default",
  userSelect: "none" as const,
};

const primaryButtonStyle = {
  ...secondaryButtonStyle,
  backgroundColor: "#2563eb",
  color: "white",
};

const stopButtonStyle = {
  ...secondaryButtonStyle,
  backgroundColor: "#7f1d1d",
  color: "#fecaca",
};

render(() => <Chat />, mainWindow);
await app.run();
await flushHistoryPersist();
