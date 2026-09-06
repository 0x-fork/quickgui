import { SecureStorage } from "@quickgui/native";

const DEEPSEEK_SERVICE = process.env.QUICKGUI_AI_CHAT_KEYCHAIN_SERVICE?.trim() || "dev.quickgui.ai-chat-example";
const DEEPSEEK_ACCOUNT = "DeepSeek API key";

export async function loadDeepSeekApiKey(): Promise<string> {
  return (await SecureStorage.getText(DEEPSEEK_SERVICE, DEEPSEEK_ACCOUNT))?.trim() ?? "";
}

export async function saveDeepSeekApiKey(apiKey: string): Promise<void> {
  const value = apiKey.trim();
  if (!value) throw new Error("The DeepSeek API key must not be empty");
  if (value.includes("\n") || value.includes("\r")) throw new Error("The DeepSeek API key must be a single line");
  if (!await SecureStorage.setText(DEEPSEEK_SERVICE, DEEPSEEK_ACCOUNT, value)) {
    throw new Error("Unable to save the DeepSeek API key");
  }
}

export async function deleteDeepSeekApiKey(): Promise<void> {
  await SecureStorage.delete(DEEPSEEK_SERVICE, DEEPSEEK_ACCOUNT);
}
