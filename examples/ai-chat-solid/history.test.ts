import { describe, expect, test } from "bun:test";
import { mkdtemp, rm, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  createConversation,
  loadConversationHistory,
  parseConversationHistory,
  saveConversationHistory,
  titleFromPrompt,
} from "./history.ts";

describe("AI chat conversation history", () => {
  test("creates a fresh conversation with the native Markdown welcome", () => {
    const conversation = createConversation(7, 11, 123);

    expect(conversation).toMatchObject({
      id: 7,
      title: "New chat",
      draft: "",
      createdAt: 123,
      updatedAt: 123,
    });
    expect(conversation.messages).toHaveLength(1);
    expect(conversation.messages[0]).toMatchObject({
      id: 11,
      role: "assistant",
    });
    expect(conversation.messages[0]!.content).toContain("QuickGUI AI Chat");
  });

  test("derives compact titles without splitting Unicode characters", () => {
    expect(titleFromPrompt("  Explain\n\tretained   rendering  ")).toBe(
      "Explain retained rendering",
    );
    expect(titleFromPrompt("🙂🙂🙂", 2)).toBe("🙂🙂…");
  });

  test("restores safe history, sorts recent conversations, and finalizes interrupted streams", () => {
    const history = parseConversationHistory({
      version: 1,
      activeConversationId: 3,
      conversations: [
        {
          id: 2,
          title: "Older",
          draft: "unfinished question",
          createdAt: 10,
          updatedAt: 20,
          messages: [{ id: 4, role: "user", content: "Hello" }],
        },
        {
          id: 3,
          title: "Newest",
          createdAt: 11,
          updatedAt: 30,
          messages: [{ id: 5, role: "assistant", content: "", streaming: true }],
        },
      ],
    });

    expect(history?.activeConversationId).toBe(3);
    expect(history?.conversations.map((conversation) => conversation.id)).toEqual([3, 2]);
    expect(history?.conversations[0]?.messages[0]).toMatchObject({
      failed: true,
      content: "_Generation was interrupted when the app closed._",
    });
    expect(history?.conversations[0]?.messages[0]?.streaming).toBeUndefined();
    expect(history?.conversations[1]?.draft).toBe("unfinished question");
  });

  test("restores legacy conversations without drafts", () => {
    const history = parseConversationHistory({
      version: 1,
      activeConversationId: 1,
      conversations: [
        {
          id: 1,
          title: "Legacy",
          createdAt: 10,
          updatedAt: 10,
          messages: [{ id: 1, role: "user", content: "Saved before draft persistence" }],
        },
      ],
    });

    expect(history?.conversations[0]?.draft).toBe("");
  });

  test("rejects malformed history instead of exposing it to the UI", () => {
    expect(parseConversationHistory({ version: 2, conversations: [] })).toBeUndefined();
    expect(
      parseConversationHistory({
        version: 1,
        activeConversationId: 1,
        conversations: [{ id: -1, messages: [] }],
      }),
    ).toBeUndefined();
  });

  test("atomically saves and reloads per-conversation drafts", async () => {
    const directory = await mkdtemp(join(tmpdir(), "quickgui-ai-chat-history-"));
    const path = join(directory, "history.json");
    const conversation = createConversation(1, 1, 123);
    conversation.draft = "persist this draft";

    try {
      await saveConversationHistory(
        { activeConversationId: conversation.id, conversations: [conversation] },
        path,
      );

      expect(await loadConversationHistory(path)).toMatchObject({
        activeConversationId: 1,
        conversations: [{ id: 1, draft: "persist this draft" }],
      });
      if (process.platform !== "win32") {
        expect((await stat(path)).mode & 0o777).toBe(0o600);
      }
    } finally {
      await rm(directory, { force: true, recursive: true });
    }
  });
});
