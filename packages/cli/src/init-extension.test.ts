import { afterEach, describe, expect, test } from "bun:test";
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { parseCliArgs } from "./args.ts";
import { loadConfig } from "./config.ts";
import { initExtension, type ExtensionType } from "./init-extension.ts";

const temporary: string[] = [];
function destination(name = "my-extension"): string {
  const root = mkdtempSync(join(tmpdir(), "quickgui-init-extension-"));
  temporary.push(root);
  return join(root, name);
}
afterEach(() => {
  for (const directory of temporary.splice(0)) rmSync(directory, { recursive: true, force: true });
});

describe("init-extension", () => {
  test("defaults to a pure Go extension and accepts native identity options", () => {
    expect(parseCliArgs(["init-extension"])).toEqual({
      command: "init-extension",
      directory: "quickgui-extension",
      type: "go",
      install: true,
    });
    for (const type of ["zig", "rust"] as const) {
      expect(
        parseCliArgs([
          "init-extension",
          "my-tool",
          `--type=${type}`,
          "--module",
          "github.com/acme/tool",
          "--name",
          "acme-tool",
          "--npm-package",
          "@acme/tool-native",
          "--no-install",
        ]),
      ).toEqual({
        command: "init-extension",
        directory: "my-tool",
        type,
        install: false,
        module: "github.com/acme/tool",
        name: "acme-tool",
        npmPackage: "@acme/tool-native",
      });
    }
    expect(parseCliArgs(["init-extension", "--help"])).toEqual({
      command: "help",
      topic: "init-extension",
    });
    expect(parseCliArgs(["help", "init-extension"])).toEqual({
      command: "help",
      topic: "init-extension",
    });
  });

  test("rejects ambiguous or unsupported arguments", () => {
    for (const args of [
      ["--type", "swift"],
      ["--type"],
      ["--type", "go", "--type", "rust"],
      ["--npm-package", "@acme/native"],
      ["one", "two"],
      ["--no-install=true"],
    ])
      expect(() => parseCliArgs(["init-extension", ...args])).toThrow();
  });

  for (const type of ["go", "zig", "rust"] as const) {
    test(`creates a usable ${type} project with a configured demo`, async () => {
      const directory = destination("My Extension");
      await initExtension({
        directory,
        type,
        install: false,
        module: "github.com/acme/notice",
        ...(type === "go" ? {} : { npmPackage: "@acme/notice-native" }),
      });
      const config = await loadConfig(directory);
      expect(config.entry).toBe(join(directory, "cmd/demo"));
      expect(config.name).toBe("my-extension demo");
      expect(readFileSync(join(directory, "go.mod"), "utf8")).toContain(
        "module github.com/acme/notice",
      );
      expect(readFileSync(join(directory, "cmd/demo/view.go"), "utf8")).toContain(
        'extension "github.com/acme/notice"',
      );
      expect(readFileSync(join(directory, "extension.go"), "utf8")).toContain(
        "package myextension",
      );
      const pkg = JSON.parse(readFileSync(join(directory, "package.json"), "utf8"));
      const cli = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
      expect(pkg.private).toBe(true);
      expect(pkg.devDependencies["@quickgui/cli"]).toBe(`^${cli.version}`);
      expect(pkg.scripts.build).toBe("bun scripts/build.ts");
      expect(existsSync(join(directory, ".gitignore"))).toBe(true);
      expect(existsSync(join(directory, "node_modules"))).toBe(false);
      expect(existsSync(join(directory, "go.sum"))).toBe(false);
      expect(existsSync(join(directory, "quickgui.extension.json"))).toBe(type !== "go");
      expect(existsSync(join(directory, "native"))).toBe(type !== "go");
      expect(existsSync(join(directory, "artifacts"))).toBe(type !== "go");
      if (type !== "go") {
        const manifest = JSON.parse(
          readFileSync(join(directory, "quickgui.extension.json"), "utf8"),
        );
        const artifact = JSON.parse(
          readFileSync(join(directory, "artifacts/package.json"), "utf8"),
        );
        expect(manifest).toEqual({
          schema: 1,
          abi: 1,
          name: "my-extension",
          package: "@acme/notice-native",
          version: "0.1.0",
          library: "quickgui_my_extension",
        });
        expect(artifact.name).toBe(manifest.package);
        expect(artifact.version).toBe(manifest.version);
        expect(readFileSync(join(directory, "scripts/dev.ts"), "utf8")).toContain(
          "QUICKGUI_EXTENSION_DIR: directory",
        );
      }
      for (const filename of readdirSync(directory, { recursive: true }) as string[]) {
        if (/\.(go|ts|toml|json|md|rs|zig|mbt|mod|pkg|work|c|h)$/.test(filename))
          expect(readFileSync(join(directory, filename), "utf8")).not.toMatch(/\{\{[A-Z_]+\}\}/);
        expect(filename.endsWith(".mjs")).toBe(false);
      }
    });
  }

  test("preserves an existing project or file", async () => {
    const directory = destination();
    await initExtension({ directory, type: "go", install: false });
    writeFileSync(join(directory, "extension.go"), "user changes\n");
    await expect(initExtension({ directory, type: "zig", install: false })).rejects.toThrow(
      "not empty",
    );
    expect(readFileSync(join(directory, "extension.go"), "utf8")).toBe("user changes\n");
    const file = destination("existing.go");
    writeFileSync(file, "keep me");
    await expect(initExtension({ directory: file, type: "go", install: false })).rejects.toThrow(
      "not a directory",
    );
    expect(readFileSync(file, "utf8")).toBe("keep me");
  });

  test("validates replacements before writing anything", async () => {
    for (const options of [
      { name: "../../outside" },
      { name: "host" },
      { name: "terminal" },
      { name: "UPPER" },
      { name: "a".repeat(65) },
      { name: "" },
      { module: "github.com/acme/../outside" },
      { module: 'github.com/acme/evil"\n' },
      { module: "github.com/acme/repo/" },
      { npmPackage: "@Acme/native" },
      { npmPackage: "@acme/native/extra" },
      { type: "bad" as ExtensionType },
    ]) {
      const directory = destination();
      await expect(
        initExtension({ directory, type: "zig", install: false, ...options }),
      ).rejects.toThrow();
      expect(existsSync(directory)).toBe(false);
    }
  });

  test("normalizes directory names and avoids Go package keywords", async () => {
    for (const [directoryName, packageName] of [
      ["123 Tools", "extension123tools"],
      ["type", "typeext"],
      ["main", "mainext"],
    ]) {
      const directory = destination(directoryName);
      await initExtension({ directory, type: "go", install: false });
      expect(readFileSync(join(directory, "extension.go"), "utf8")).toContain(
        `package ${packageName}\n`,
      );
    }
  });

  for (const type of ["zig"]) {
    test(`ships the canonical standalone C ABI for ${type}`, () => {
      expect(
        readFileSync(
          new URL(`../templates/extension/${type}/native/quickgui_extension.h`, import.meta.url),
          "utf8",
        ),
      ).toBe(
        readFileSync(new URL("../../../include/quickgui_extension.h", import.meta.url), "utf8"),
      );
    });
  }

  test("the real CLI creates a project without invoking installers", async () => {
    const directory = destination("with spaces");
    const child = Bun.spawn(
      [
        process.execPath,
        resolve(import.meta.dir, "cli.ts"),
        "init-extension",
        directory,
        "--type",
        "rust",
        "--no-install",
      ],
      { env: { ...process.env, PATH: "" }, stdout: "pipe", stderr: "pipe" },
    );
    const [status, stdout, stderr] = await Promise.all([
      child.exited,
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
    ]);
    expect(stderr).toBe("");
    expect(status).toBe(0);
    expect(stdout).toContain("Created rust extension");
    expect(stdout).toContain("bun run dev");
    expect(existsSync(join(directory, "native/src/lib.rs"))).toBe(true);
  });
});
