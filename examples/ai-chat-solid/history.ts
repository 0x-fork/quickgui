import { mkdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import { dirname, join } from "node:path";

export type ChatRole = "user" | "assistant";

export interface ChatMessage {
  id: number;
  role: ChatRole;
  content: string;
  streaming?: boolean;
  failed?: boolean;
}

export interface Conversation {
  id: number;
  title: string;
  messages: ChatMessage[];
  draft: string;
  createdAt: number;
  updatedAt: number;
}

export interface ConversationHistory {
  activeConversationId: number;
  conversations: Conversation[];
}

const HISTORY_VERSION = 1;
const MAX_STORED_CONVERSATIONS = 100;
const MAX_STORED_MESSAGES = 101;
const MAX_STORED_CONTENT_CHARACTERS = 256_000;
const MAX_STORED_DRAFT_CHARACTERS = 16_000;
const DEFAULT_TITLE = "New chat";

const WELCOME_CONTENT =
  "# QuickGUI AI Chat\n\nAsk a question to stream a response from **DeepSeek**. Markdown is parsed and rendered by QuickGUI core—not a webview.";

export function createConversation(
  id: number,
  welcomeMessageId: number,
  now = Date.now(),
): Conversation {
  return {
    id,
    title: DEFAULT_TITLE,
    draft: "",
    messages: [
      {
        id: welcomeMessageId,
        role: "assistant",
        content: WELCOME_CONTENT,
      },
    ],
    createdAt: now,
    updatedAt: now,
  };
}

export function titleFromPrompt(prompt: string, maximumCharacters = 42): string {
  const normalized = prompt.replace(/\s+/g, " ").trim();
  if (!normalized) return DEFAULT_TITLE;
  const characters = Array.from(normalized);
  if (characters.length <= maximumCharacters) return normalized;
  return `${characters.slice(0, maximumCharacters).join("")}…`;
}

export function parseConversationHistory(value: unknown): ConversationHistory | undefined {
  if (!isRecord(value) || value.version !== HISTORY_VERSION || !Array.isArray(value.conversations)) {
    return undefined;
  }

  const conversations = value.conversations
    .slice(0, MAX_STORED_CONVERSATIONS)
    .map(parseConversation)
    .filter((conversation): conversation is Conversation => conversation !== undefined)
    .sort((left, right) => right.updatedAt - left.updatedAt);
  if (conversations.length === 0) return undefined;

  const requestedActiveId = finitePositiveInteger(value.activeConversationId);
  const activeConversationId = conversations.some(
    (conversation) => conversation.id === requestedActiveId,
  )
    ? requestedActiveId!
    : conversations[0]!.id;
  return { activeConversationId, conversations };
}

export async function loadConversationHistory(
  path = conversationHistoryPath(),
): Promise<ConversationHistory | undefined> {
  try {
    return parseConversationHistory(JSON.parse(await readFile(path, "utf8")));
  } catch (error) {
    if (isMissingFile(error) || error instanceof SyntaxError) return undefined;
    console.error(`Unable to load chat history from ${path}`, error);
    return undefined;
  }
}

export async function saveConversationHistory(
  history: ConversationHistory,
  path = conversationHistoryPath(),
): Promise<void> {
  await mkdir(dirname(path), { recursive: true });
  const temporaryPath = `${path}.${process.pid}.tmp`;
  const payload = JSON.stringify(
    {
      version: HISTORY_VERSION,
      activeConversationId: history.activeConversationId,
      conversations: history.conversations.slice(0, MAX_STORED_CONVERSATIONS),
    },
    null,
    2,
  );
  await writeFile(temporaryPath, payload, { encoding: "utf8", mode: 0o600 });
  try {
    await rename(temporaryPath, path);
  } catch (error) {
    await rm(temporaryPath, { force: true });
    throw error;
  }
}

export function conversationHistoryPath(): string {
  const override = process.env.QUICKGUI_AI_CHAT_DATA_DIR?.trim();
  if (override) return join(override, "history.json");

  if (process.platform === "darwin") {
    return join(homedir(), "Library", "Application Support", "QuickGUI AI Chat", "history.json");
  }
  if (process.platform === "win32") {
    const applicationData = process.env.APPDATA?.trim() || join(homedir(), "AppData", "Roaming");
    return join(applicationData, "QuickGUI AI Chat", "history.json");
  }
  const dataHome = process.env.XDG_DATA_HOME?.trim() || join(homedir(), ".local", "share");
  return join(dataHome, "quickgui-ai-chat", "history.json");
}

function parseConversation(value: unknown): Conversation | undefined {
  if (!isRecord(value) || !Array.isArray(value.messages)) return undefined;
  const id = finitePositiveInteger(value.id);
  const createdAt = finiteNonNegativeNumber(value.createdAt);
  const updatedAt = finiteNonNegativeNumber(value.updatedAt);
  if (id === undefined || createdAt === undefined || updatedAt === undefined) return undefined;

  const messages = value.messages
    .slice(-MAX_STORED_MESSAGES)
    .map(parseMessage)
    .filter((message): message is ChatMessage => message !== undefined);
  if (messages.length === 0) return undefined;

  const title = typeof value.title === "string" ? titleFromPrompt(value.title) : DEFAULT_TITLE;
  const draft = typeof value.draft === "string"
    ? value.draft.slice(0, MAX_STORED_DRAFT_CHARACTERS)
    : "";
  return {
    id,
    title,
    draft,
    messages,
    createdAt,
    updatedAt: Math.max(createdAt, updatedAt),
  };
}

function parseMessage(value: unknown): ChatMessage | undefined {
  if (!isRecord(value)) return undefined;
  const id = finitePositiveInteger(value.id);
  if (
    id === undefined ||
    (value.role !== "user" && value.role !== "assistant") ||
    typeof value.content !== "string"
  ) {
    return undefined;
  }

  const content = value.content.slice(0, MAX_STORED_CONTENT_CHARACTERS);
  if (value.streaming === true) {
    return {
      id,
      role: value.role,
      content: content || "_Generation was interrupted when the app closed._",
      failed: true,
    };
  }

  return {
    id,
    role: value.role,
    content,
    ...(value.failed === true ? { failed: true } : {}),
  };
}

function finitePositiveInteger(value: unknown): number | undefined {
  return typeof value === "number" && Number.isSafeInteger(value) && value > 0 ? value : undefined;
}

function finiteNonNegativeNumber(value: unknown): number | undefined {
  return typeof value === "number" && Number.isFinite(value) && value >= 0 ? value : undefined;
}

function isMissingFile(error: unknown): boolean {
  return isRecord(error) && error.code === "ENOENT";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
