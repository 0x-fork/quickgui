import { Protocol } from "./integrations.ts";

export interface SecondInstanceEvent {
  argv: readonly string[];
  cwd: string;
}

export function urlsFromArguments(arguments_: readonly string[]): string[] {
  const urls: string[] = [];
  for (const argument of arguments_) {
    try {
      const url = new URL(argument);
      if (url.protocol && url.protocol !== "http:" && url.protocol !== "https:") {
        urls.push(url.href);
      }
    } catch {
      // Ordinary CLI flags and filesystem paths are not deep links.
    }
  }
  return urls;
}

export const DeepLink = Object.freeze({
  getLaunchUrls(): readonly string[] {
    return urlsFromArguments(process.argv.slice(1));
  },
  supportsDynamicRegistration: Protocol.supportsDynamicRegistration,
  register: Protocol.register,
  unregister: Protocol.unregister,
  isRegistered: Protocol.isRegistered,
});
