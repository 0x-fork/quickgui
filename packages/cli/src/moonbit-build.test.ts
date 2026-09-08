import { expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { parseCliArgs } from "./args.ts";
import { resolveConfig } from "./config.ts";
import { initProject } from "./init.ts";
import {
  moonbitBuildEnvironment,
  moonbitBuildPlan,
  moonbitExecutable,
  moonbitExtensions,
  moonbitModuleName,
  requireMoonbitExecutable,
} from "./moonbit-build.ts";
import { hostTarget, supportedTargets } from "./targets.ts";
import { shouldIgnoreChange } from "./dev.ts";

test("MoonBit is opt-in and Go remains the default", () => {
  const input = { name: "Test", identifier: "dev.test.app" };
  const go = resolveConfig(input, "/tmp/project");
  const moonbit = resolveConfig({ ...input, frontend: "moonbit" }, "/tmp/project");
  expect(go.frontend).toBe("go");
  expect(go.entry).toBe("/tmp/project");
  expect(moonbit.entry).toBe("/tmp/project/main");
  expect(parseCliArgs(["init", "demo", "--frontend", "moonbit", "--no-install"])).toMatchObject({
    command: "init",
    frontend: "moonbit",
    install: false,
  });
  expect(() => parseCliArgs(["init", "--frontend", "javascript"])).toThrow(
    "expected go or moonbit",
  );
  expect(() =>
    resolveConfig({ ...input, frontend: "moonbit", native: { tags: ["foo"] } }, "/tmp"),
  ).toThrow("Go build tags");
  expect(() => resolveConfig({ ...input, native: { extensions: [] } }, "/tmp")).toThrow(
    "Go extensions are discovered",
  );
});

test("MoonBit uses fast native development builds and optimized C release builds", () => {
  for (const target of supportedTargets) {
    const supported = ["darwin-arm64", "linux-x64", "windows-x64"].includes(target);
    expect(moonbitBuildEnvironment("development", target, {})).toEqual({
      MOONBIT_NEW_NATIVE: supported ? "1" : "0",
    });
    // An inherited opt-in must not select an unsupported backend or alter release output.
    expect(moonbitBuildEnvironment("development", target, { MOONBIT_NEW_NATIVE: "1" })).toEqual({
      MOONBIT_NEW_NATIVE: supported ? "1" : "0",
    });
    expect(moonbitBuildEnvironment("production", target, { MOONBIT_NEW_NATIVE: "1" })).toEqual({
      MOONBIT_NEW_NATIVE: "0",
    });
    expect(
      moonbitBuildEnvironment("development", target, {
        MOONBIT_NEW_NATIVE: "0",
        MOON_HOME: "/custom/moon",
        PATH: "/custom/bin",
      }),
    ).toEqual({ MOONBIT_NEW_NATIVE: "0", MOON_HOME: "/custom/moon", PATH: "/custom/bin" });
  }
});

test("MoonBit build plans preserve the incremental cache and native executable packaging", () => {
  const config = resolveConfig(
    { name: "Moon", identifier: "dev.test.moon", frontend: "moonbit" },
    "/tmp/moon",
  );
  const options = {
    config,
    mode: "development" as const,
    target: hostTarget(),
    executablePath: "/tmp/Moon",
    fonts: [],
  };
  const plan = moonbitBuildPlan(options);
  expect(plan.argv.slice(0, 2)).toEqual(["moon", "build"]);
  expect(plan.argv).toContain("--debug");
  expect(plan.env).toEqual(moonbitBuildEnvironment("development", options.target));
  expect(plan.targetDir).toBe("/tmp/moon/.quickgui/moonbit");
  expect(plan.argv.join(" ")).not.toMatch(/cargo|go build/);
  const release = moonbitBuildPlan({ ...options, mode: "production" });
  expect(release.argv).toContain("--release");
  expect(release.env.MOONBIT_NEW_NATIVE).toBe("0");
  expect(() =>
    moonbitBuildPlan({
      ...options,
      target: hostTarget() === "darwin-arm64" ? "linux-x64" : "darwin-arm64",
    }),
  ).toThrow("match the build host");
  for (const dir of ["_build", ".mooncakes", ".repos", ".quickgui"]) {
    expect(shouldIgnoreChange("/tmp/moon", `/tmp/moon/${dir}/file`, "/tmp/moon/dist")).toBe(true);
  }
});

test("executable selection identifies exactly the requested package and rejects stale/foreign artifacts", () => {
  const build = "/tmp/build";
  const good = { root: "acme/app", rel: "main", artifact: "/tmp/build/acme/app/main/main.mi" };
  expect(
    moonbitExecutable({ packages: [good, { ...good, rel: "other" }] }, "acme/app", "main", build),
  ).toEndWith("main.exe");
  expect(() => moonbitExecutable({ packages: [] }, "acme/app", "main", build)).toThrow(
    "requested executable",
  );
  expect(() => moonbitExecutable({ packages: [good, good] }, "acme/app", "main", build)).toThrow(
    "requested executable",
  );
  expect(() =>
    moonbitExecutable(
      { packages: [{ ...good, artifact: "/foreign/main.mi" }] },
      "acme/app",
      "main",
      build,
    ),
  ).toThrow("outside");
});

test("MoonBit scaffold has a fluent component, TOML config, and no Go files", async () => {
  const root = mkdtempSync(join(tmpdir(), "quickgui-moonbit-init-"));
  try {
    const project = join(root, "demo");
    await initProject({
      directory: project,
      frontend: "moonbit",
      install: false,
      name: 'Moon "App"',
    });
    const config = Bun.TOML.parse(readFileSync(join(project, "quickgui.toml"), "utf8")) as Record<
      string,
      unknown
    >;
    expect(config.frontend).toBe("moonbit");
    expect(config.name).toBe('Moon "App"');
    expect(moonbitModuleName(project)).toBe("myapp/moon-app");
    requireMoonbitExecutable(join(project, "main"));
    const source = readFileSync(join(project, "main/main.mbt"), "utf8");
    expect(source).toContain(".size_full()");
    expect(source).toContain("..if count() >= 5");
    expect(source).toContain("@reactive.create_signal(0)");
    expect(source).not.toContain(".child(");
    expect(source).not.toContain("{{");
    expect(() => readFileSync(join(project, "go.mod"))).toThrow();
    await expect(
      initProject({ directory: project, frontend: "moonbit", install: false }),
    ).rejects.toThrow("not empty");
    writeFileSync(join(project, "main/moon.pkg"), '// pkgtype(kind: "executable")\n');
    expect(() => requireMoonbitExecutable(join(project, "main"))).toThrow("must declare");
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("MoonBit reads and validates manifests from selected extension directories", () => {
  const root = mkdtempSync(join(tmpdir(), "quickgui-moonbit-extension-"));
  try {
    const path = join(root, "quickgui.extension.json");
    writeFileSync(
      path,
      JSON.stringify({
        schema: 1,
        abi: 1,
        name: "example",
        package: "@acme/example",
        version: "1.2.3",
        library: "quickgui_example",
      }),
    );
    const extensions = moonbitExtensions([root]);
    expect(extensions[0]?.name).toBe("example");
    expect(moonbitExtensions([root, root])).toEqual(extensions);
    expect(moonbitExtensions([])).toEqual([]);
    expect(() => moonbitExtensions([path])).toThrow("Expected an extension directory");
    expect(() => moonbitExtensions([join(root, "missing")])).toThrow(
      "Expected an extension directory",
    );
    const otherDirectory = join(root, "other");
    mkdirSync(otherDirectory);
    const other = join(otherDirectory, "quickgui.extension.json");
    expect(() => moonbitExtensions([otherDirectory])).toThrow(
      "Expected a quickgui.extension.json file",
    );
    mkdirSync(other);
    expect(() => moonbitExtensions([otherDirectory])).toThrow(
      "Expected a quickgui.extension.json file",
    );
    rmSync(other, { recursive: true });
    writeFileSync(other, readFileSync(path, "utf8").replace("1.2.3", "2.0.0"));
    expect(() => moonbitExtensions([root, otherDirectory])).toThrow("Conflicting requirements");
    writeFileSync(other, readFileSync(path, "utf8").replace('"abi":1', '"abi":2'));
    expect(() => moonbitExtensions([otherDirectory])).toThrow("Invalid QuickGUI extension manifest");
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
