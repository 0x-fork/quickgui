import { afterEach, describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

import { parseCliArgs } from "./args.ts";
import { macInfoPlist } from "./build.ts";
import { resolveConfig } from "./config.ts";
import { initProject } from "./init.ts";
import { hostTarget, parseTarget } from "./targets.ts";

const temporaryRoots: string[] = [];

afterEach(() => {
  for (const root of temporaryRoots.splice(0)) {
    rmSync(root, { recursive: true, force: true });
  }
});

describe("CLI arguments", () => {
  test("parses init, dev, and build options", () => {
    expect(parseCliArgs(["init", "hello", "--no-install", "--name=Hello"])).toEqual({
      command: "init",
      directory: "hello",
      install: false,
      name: "Hello",
    });
    expect(parseCliArgs(["dev", "--target", hostTarget(), "--once", "--no-launch"])).toEqual({
      command: "dev",
      project: ".",
      configFile: "quickgui.config.ts",
      target: hostTarget(),
      once: true,
      launch: false,
    });
    expect(
      parseCliArgs(["build", "--project", "demo", "--out-dir=artifacts", "--sign", "-"]),
    ).toEqual({
      command: "build",
      project: "demo",
      configFile: "quickgui.config.ts",
      outDir: "artifacts",
      signingIdentity: "-",
    });
  });

  test("rejects unknown and duplicate options", () => {
    expect(() => parseCliArgs(["dev", "--wat"])).toThrow("Unknown option");
    expect(() => parseCliArgs(["build", "--target", hostTarget(), "--target", hostTarget()])).toThrow(
      "only be specified once",
    );
    expect(() => parseTarget("plan9-x64")).toThrow("Unsupported target");
  });
});

describe("project configuration", () => {
  test("normalizes paths and creates a filesystem-safe executable name", () => {
    const root = temporaryRoot();
    const config = resolveConfig(
      {
        name: "My / Great App",
        identifier: "com.example.great-app",
        entry: "ui/main.tsx",
        resources: ["assets"],
      },
      root,
    );
    expect(config.executableName).toBe("My-Great-App");
    expect(config.entry).toBe(join(root, "ui/main.tsx"));
    expect(config.resources).toEqual([join(root, "assets")]);
    expect(config.macos.minimumSystemVersion).toBe("13.0");
  });

  test("rejects an invalid bundle identifier", () => {
    expect(() =>
      resolveConfig({ name: "Bad", identifier: "not a reverse dns identifier" }, temporaryRoot()),
    ).toThrow("Invalid application identifier");
  });
});

test("macOS metadata is escaped and complete", () => {
  const plist = macInfoPlist({
    name: "A & B",
    displayName: "A < B",
    executableName: "A-B",
    identifier: "com.example.a-b",
    version: "1.2.3",
    buildVersion: "7",
    minimumSystemVersion: "13.0",
    category: "public.app-category.developer-tools",
    iconFile: "AppIcon.icns",
  });
  expect(plist).toContain("<string>A &amp; B</string>");
  expect(plist).toContain("<string>A &lt; B</string>");
  expect(plist).toContain("<key>CFBundleExecutable</key>");
  expect(plist).toContain("<string>AppIcon.icns</string>");
});

test("project initialization renders a complete Solid scaffold", async () => {
  const root = temporaryRoot();
  const project = join(root, "sample-app");
  await initProject({
    directory: project,
    install: false,
    name: "Sample App",
    identifier: "com.example.sample-app",
  });

  expect(JSON.parse(readFileSync(join(project, "package.json"), "utf8"))).toMatchObject({
    name: "sample-app",
    scripts: { dev: "quickgui dev", build: "quickgui build" },
  });
  expect(readFileSync(join(project, "quickgui.config.ts"), "utf8")).toContain(
    'identifier: "com.example.sample-app"',
  );
  expect(readFileSync(join(project, "src/app.tsx"), "utf8")).toContain(
    'from "@quickgui/solid"',
  );
  expect(readFileSync(join(project, ".gitignore"), "utf8")).toContain(".quickgui");
  expect(readFileSync(join(project, "README.md"), "utf8")).not.toContain("{{");
});

test("project initialization never overwrites a non-empty destination", async () => {
  const root = temporaryRoot();
  writeFileSync(join(root, "keep.txt"), "mine");
  await expect(initProject({ directory: root, install: false })).rejects.toThrow("is not empty");
  expect(readFileSync(join(root, "keep.txt"), "utf8")).toBe("mine");
});

function temporaryRoot(): string {
  const root = mkdtempSync(join(tmpdir(), "quickgui-cli-test-"));
  temporaryRoots.push(root);
  return root;
}
