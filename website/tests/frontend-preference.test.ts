import { expect, test } from "bun:test";
import { readFrontendPreference, rememberFrontend } from "../src/lib/frontend-preference";
import { resolveDocsRoute } from "../src/lib/docs-routing";
import { switchDocsFrontend } from "../src/lib/docs";

test("TypeScript selection persists and resolves the same guide or component", () => {
  expect(readFrontendPreference("theme=dark; quickgui-frontend=typescript")).toBe("typescript");
  expect(resolveDocsRoute(undefined, undefined, "typescript")).toEqual({
    kind: "redirect",
    path: "/docs/typescript",
  });
  expect(switchDocsFrontend("/docs/go/components/slider", "typescript")).toBe(
    "/docs/typescript/components/slider",
  );
  expect(readFrontendPreference("quickgui-frontend=unknown")).toBe("go");
  const previous = Object.getOwnPropertyDescriptor(globalThis, "document");
  const document = { cookie: "" };
  Object.defineProperty(globalThis, "document", { configurable: true, value: document });
  try {
    rememberFrontend("typescript");
    expect(document.cookie).toContain("quickgui-frontend=typescript; Path=/;");
    expect(document.cookie).toContain("SameSite=Lax");
  } finally {
    if (previous) Object.defineProperty(globalThis, "document", previous);
    else Reflect.deleteProperty(globalThis, "document");
  }
});
