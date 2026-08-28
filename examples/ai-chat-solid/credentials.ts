const SECURITY_TOOL = "/usr/bin/security";
const DEEPSEEK_SERVICE =
  process.env.QUICKGUI_AI_CHAT_KEYCHAIN_SERVICE?.trim() || "dev.quickgui.ai-chat-example";
const DEEPSEEK_ACCOUNT = "DeepSeek API key";
const DEEPSEEK_LABEL = "QuickGUI AI Chat DeepSeek API key";
const ITEM_NOT_FOUND_EXIT_CODE = 44;

interface SecurityResult {
  exitCode: number;
  stdout: string;
  stderr: string;
}

export async function loadDeepSeekApiKey(): Promise<string> {
  if (process.platform !== "darwin") return "";
  const result = await runSecurity([
    "find-generic-password",
    "-a",
    DEEPSEEK_ACCOUNT,
    "-s",
    DEEPSEEK_SERVICE,
    "-w",
  ]);
  if (result.exitCode === ITEM_NOT_FOUND_EXIT_CODE) return "";
  assertSecuritySucceeded("read the DeepSeek API key", result);
  return result.stdout.trim();
}

export async function saveDeepSeekApiKey(apiKey: string): Promise<void> {
  if (process.platform !== "darwin") {
    throw new Error("Secure provider-key persistence is currently available on macOS only");
  }
  const value = apiKey.trim();
  if (!value) throw new Error("The DeepSeek API key must not be empty");
  if (value.includes("\n") || value.includes("\r")) {
    throw new Error("The DeepSeek API key must be a single line");
  }
  const result = await runSecurityWithPassword(
    [
      "add-generic-password",
      "-U",
      "-a",
      DEEPSEEK_ACCOUNT,
      "-s",
      DEEPSEEK_SERVICE,
      "-l",
      DEEPSEEK_LABEL,
      "-w",
    ],
    value,
  );
  assertSecuritySucceeded("save the DeepSeek API key", result);
}

export async function deleteDeepSeekApiKey(): Promise<void> {
  if (process.platform !== "darwin") return;
  const result = await runSecurity([
    "delete-generic-password",
    "-a",
    DEEPSEEK_ACCOUNT,
    "-s",
    DEEPSEEK_SERVICE,
  ]);
  if (result.exitCode === ITEM_NOT_FOUND_EXIT_CODE) return;
  assertSecuritySucceeded("remove the DeepSeek API key", result);
}

async function runSecurity(arguments_: string[]): Promise<SecurityResult> {
  const child = Bun.spawn([SECURITY_TOOL, ...arguments_], {
    stdin: "ignore",
    stdout: "pipe",
    stderr: "pipe",
  });
  const [exitCode, stdout, stderr] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);
  return { exitCode, stdout, stderr };
}

async function runSecurityWithPassword(
  arguments_: string[],
  password: string,
): Promise<SecurityResult> {
  let wrotePassword = false;
  const terminal = new Bun.Terminal({
    data(currentTerminal) {
      // Respond only after `security` emits its prompt. New items ask for the password twice while
      // updates consume the first line and exit. Deliberately do not retain terminal output because
      // the tool echoes typed input even though the password never enters argv or a shell.
      if (!wrotePassword) {
        wrotePassword = true;
        currentTerminal.write(`${password}\r${password}\r`);
      }
    },
  });
  const child = Bun.spawn([SECURITY_TOOL, ...arguments_], { terminal });
  let timedOut = false;
  const timeout = setTimeout(() => {
    timedOut = true;
    child.kill();
  }, 5_000);
  try {
    const exitCode = await child.exited;
    if (timedOut) {
      return { exitCode: 1, stdout: "", stderr: "macOS Keychain did not accept the password" };
    }
    if (!wrotePassword) {
      return { exitCode: 1, stdout: "", stderr: "macOS Keychain did not request the password" };
    }
    return {
      exitCode,
      stdout: "",
      stderr: exitCode === 0 ? "" : `macOS Keychain exited with code ${exitCode}`,
    };
  } finally {
    clearTimeout(timeout);
    terminal.close();
  }
}

function assertSecuritySucceeded(action: string, result: SecurityResult): void {
  if (result.exitCode === 0) return;
  const detail = result.stderr.trim() || result.stdout.trim() || `exit code ${result.exitCode}`;
  throw new Error(`Unable to ${action}: ${detail}`);
}
