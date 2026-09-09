import { describe, expect, test } from "bun:test";
import { writeFile } from "node:fs/promises";

import {
  AgentError,
  MAX_PROMPT_DIFF_CHARACTERS,
  agentCommand,
  boundDiff,
  buildPrompt,
  generateCommitMessage,
  parseMessage,
  type AvailableAgent,
  type ProcessRunner,
} from "./commit-message.ts";

const codex: AvailableAgent = {
  id: "codex",
  label: "Codex",
  command: "codex",
  description: "",
  executable: "/bin/codex",
};
const claude: AvailableAgent = {
  id: "claude",
  label: "Claude",
  command: "claude",
  description: "",
  executable: "/bin/claude",
};

describe("commit message prompts", () => {
  test("include conventions, files, and the diff in a fenced block", () => {
    const prompt = buildPrompt({
      repositoryName: "quickgui",
      branch: "main",
      diff: "diff --git a/x b/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n",
      files: ["M x"],
      recentSubjects: ["fix: parse renames", "feat: add worktrees"],
      hint: "mention ticket QG-12",
    });
    expect(prompt).toContain("Repository: quickgui (branch main).");
    expect(prompt).toContain("- fix: parse renames");
    expect(prompt).toContain("Instruction from the author: mention ticket QG-12");
    expect(prompt).toContain("Files changed (1):\n- M x");
    expect(prompt).toContain("```diff\ndiff --git a/x b/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n```");
    expect(prompt).not.toContain("amends");
  });

  test("bound the diff by whole files and say how many were left out", () => {
    const file = (name: string, size: number) =>
      `diff --git a/${name} b/${name}\n${"+x\n".repeat(size)}`;
    const diff = file("a", 10) + file("b", 10) + file("c", 10);
    const bounded = boundDiff(diff, file("a", 10).length + file("b", 10).length + 1);
    expect(bounded.omitted).toBe(1);
    expect(bounded.text).toBe(file("a", 10) + file("b", 10));
    const huge = boundDiff(file("big", MAX_PROMPT_DIFF_CHARACTERS), 100);
    expect(huge.omitted).toBe(0);
    expect(huge.text.endsWith("… (diff truncated)\n")).toBe(true);
    const prompt = buildPrompt({
      repositoryName: "r",
      diff,
      files: ["M a", "M b", "M c"],
      recentSubjects: [],
    });
    expect(prompt).not.toContain("left out");
  });
});

describe("commit message parsing", () => {
  test("strips fences, labels, and quotes and splits subject from body", () => {
    expect(
      parseMessage("```\nfix: handle renames\n\nRenames were parsed as deletions.\n```"),
    ).toEqual({
      subject: "fix: handle renames",
      body: "Renames were parsed as deletions.",
    });
    expect(parseMessage('Commit message: "Add worktree view"')).toEqual({
      subject: "Add worktree view",
      body: "",
    });
    expect(parseMessage("\n\nSubject only\n\n")).toEqual({ subject: "Subject only", body: "" });
    expect(parseMessage("   ")).toBeUndefined();
  });
});

describe("running an agent", () => {
  test("invokes codex non-interactively with a read-only sandbox and reads its output file", async () => {
    const calls: { command: readonly string[]; stdin: string }[] = [];
    const run: ProcessRunner = async (command, options) => {
      calls.push({ command, stdin: options.stdin });
      await writeFile(
        command[command.indexOf("-o") + 1]!,
        "feat: add commit generation\n\nUses the local agent.\n",
      );
      return { stdout: "progress noise", stderr: "", exitCode: 0 };
    };
    const message = await generateCommitMessage(
      codex,
      { repositoryName: "r", diff: "diff --git a/x b/x\n", files: ["M x"], recentSubjects: [] },
      "/tmp",
      { run },
    );
    expect(message).toMatchObject({
      subject: "feat: add commit generation",
      body: "Uses the local agent.",
      agent: "codex",
    });
    const command = calls[0]!.command;
    expect(command.slice(0, 2)).toEqual(["/bin/codex", "exec"]);
    expect(command).toContain("--sandbox");
    expect(command[command.indexOf("--sandbox") + 1]).toBe("read-only");
    expect(command).toContain("--ephemeral");
    expect(command.at(-1)).toBe("-");
    expect(calls[0]!.stdin).toContain("Files changed (1)");
  });

  test("invokes claude in print mode with tools disabled and reads stdout", async () => {
    const run: ProcessRunner = async () => ({
      stdout: "Improve diff parsing\n",
      stderr: "",
      exitCode: 0,
    });
    const message = await generateCommitMessage(
      claude,
      { repositoryName: "r", diff: "", files: [], recentSubjects: [] },
      "/tmp",
      { run },
    );
    expect(message.subject).toBe("Improve diff parsing");
    const command = agentCommand(claude, "/tmp", "/tmp/out");
    expect(command).toEqual([
      "/bin/claude",
      "-p",
      "--output-format",
      "text",
      "--tools",
      "",
      "--no-session-persistence",
      "--disable-slash-commands",
    ]);
  });

  test("reports failures with the last line the agent printed and honours cancellation", async () => {
    const failing: ProcessRunner = async () => ({
      stdout: "",
      stderr: "error: not logged in\n",
      exitCode: 1,
    });
    const failure = await generateCommitMessage(
      claude,
      { repositoryName: "r", diff: "", files: [], recentSubjects: [] },
      "/tmp",
      { run: failing },
    ).catch((error: unknown) => error);
    expect(failure).toBeInstanceOf(AgentError);
    expect((failure as AgentError).message).toBe("Claude: error: not logged in");

    const controller = new AbortController();
    const cancelling: ProcessRunner = async () => {
      controller.abort();
      return { stdout: "partial", stderr: "", exitCode: 0 };
    };
    const cancelled = await generateCommitMessage(
      claude,
      { repositoryName: "r", diff: "", files: [], recentSubjects: [] },
      "/tmp",
      {
        run: cancelling,
        signal: controller.signal,
      },
    ).catch((error: unknown) => error);
    expect((cancelled as AgentError).aborted).toBe(true);
  });
});
