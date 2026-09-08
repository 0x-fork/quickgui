import { cpSync, mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync } from "node:fs";
import { join, basename, resolve } from "node:path";
import { tmpdir } from "node:os";
import { runMoonbit } from "../packages/cli/src/moonbit-build.ts";
const root = resolve(import.meta.dir, "..");
const temporary = mkdtempSync(join(tmpdir(), "quickgui-moonbit-docs-"));
try {
  cpSync(join(root, "moonbit"), join(temporary, "sdk"), {
    recursive: true,
    filter: (path) => !["_build", ".mooncakes", ".repos", ".git"].includes(basename(path)),
  });
  mkdirSync(join(temporary, "app", "main"), { recursive: true });
  writeFileSync(join(temporary, "moon.work"), 'members = ["sdk", "app"]\n');
  const version = /version\s*=\s*"([^"]+)"/.exec(
    readFileSync(join(root, "moonbit/moon.mod"), "utf8"),
  )![1];
  writeFileSync(
    join(temporary, "app", "moon.mod"),
    `name = "quickgui/docs"\npreferred_target = "native"\nsupported_targets = "native"\nimport { "egoist/quickgui@${version}" }\n`,
  );
  writeFileSync(
    join(temporary, "app", "main", "moon.pkg"),
    'import { "egoist/quickgui/ui", "egoist/quickgui/native", "egoist/quickgui/reactive", "egoist/quickgui/protocol", "egoist/quickgui/terminal" }\npkgtype(kind: "executable")\n',
  );
  const files: string[] = [];
  for (const dir of ["en", "components/ui", "components/swift-ui"]) {
    for await (const file of new Bun.Glob("*.mdx").scan(
      join(root, "website/src/content/docs/moonbit", dir),
    ))
      files.push(join(root, "website/src/content/docs/moonbit", dir, file));
  }
  let count = 0;
  for (const path of files.sort()) {
    for (const match of readFileSync(path, "utf8").matchAll(/```moonbit\n([\s\S]*?)```/g)) {
      if (!/^fn /m.test(match[1]!)) continue;
      const source = match[1]!.replace(/\bcomponent_example\b/g, `component_example_${count}`);
      writeFileSync(
        join(temporary, "app", "main", `example-${count++}.mbt`),
        `// Source: ${path}\n${source}`,
      );
    }
  }
  const moonHome = process.env.MOON_HOME ?? join(root, "target/moonbit-toolchain");
  // These reference examples intentionally define exported-looking functions
  // without calling all of them from the one executable entry point.
  const status = await runMoonbit(
    join(temporary, "app"),
    ["check", "--warn-list", "-1"],
    "development",
    { ...process.env, MOON_HOME: moonHome, PATH: `${moonHome}/bin:${process.env.PATH ?? ""}` },
    120_000,
  );
  if (status) {
    throw new Error("MoonBit documentation examples did not compile");
  }
  console.log(`Compiled ${count} MoonBit documentation examples with the native backend`);
} finally {
  rmSync(temporary, { recursive: true, force: true });
}
