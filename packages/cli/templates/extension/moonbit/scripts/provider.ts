import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  realpathSync,
  renameSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { run, type Manifest } from "./build.ts";

export async function buildProvider(
  root: string,
  manifest: Manifest,
  output: string,
): Promise<void> {
  const moon = Bun.which(process.env.MOON ?? "moon");
  if (!moon) throw new Error("Install MoonBit and put moon on PATH (or set MOON)");
  const home = process.env.MOON_HOME ?? dirname(dirname(realpathSync(moon)));
  const toolchain = existsSync(join(home, "include/moonbit.h")) ? home : join(homedir(), ".moon");
  if (!existsSync(join(toolchain, "include/moonbit.h")))
    throw new Error("MoonBit runtime headers were not found; set MOON_HOME to your toolchain");
  const compiler = join(toolchain, "bin", process.platform === "win32" ? "moonc.exe" : "moonc");
  const native = join(root, "native");
  const target = join(native, "target");
  // The direct object backend does not emit linkable libraries yet. Compile the
  // MoonBit package to .core, generate C with explicit exports, then link a DSO.
  const env = { ...process.env, MOON_HOME: toolchain, MOONBIT_NEW_NATIVE: "0" };
  await run(
    [moon, "build", "--target", "native", "--release", "--target-dir", target],
    native,
    env,
  );
  const build = join(target, "native/release/build");
  const packages = JSON.parse(readFileSync(join(build, "all_pkgs.json"), "utf8")).packages as {
    root: string;
    rel: string;
    artifact: string;
  }[];
  const entry = packages.find(
    (pkg) => pkg.rel === "" && dirname(realpathSync(pkg.artifact)) === realpathSync(build),
  );
  if (!entry) throw new Error("MoonBit did not report the native provider's root package");
  const cores = packages.map((pkg) => pkg.artifact.replace(/\.mi$/, ".core"));
  const generated = join(target, "provider.c");
  await run(
    [
      compiler,
      "link-core",
      ...cores,
      "-main",
      entry.root,
      "-exported_functions",
      "qge_invoke,qge_shutdown",
      "-target",
      "native",
      "-o",
      generated,
    ],
    native,
    env,
  );
  writeFileSync(
    join(target, "build_info.h"),
    `// Generated from quickgui.extension.json.\n#define QGE_NAME ${JSON.stringify(manifest.name)}\n#define QGE_VERSION ${JSON.stringify(manifest.version)}\n`,
  );

  // Compile a private runtime from the same toolchain as the generated C.
  // Prebuilt executable runtime objects need not be PIC and can export symbols
  // that collide with a MoonBit app or another provider's object-layout table.
  const sources = [
    generated,
    join(native, "bridge.c"),
    ...["runtime.c", "env.c", "utf.c", "sync_io.c"].map((file) =>
      join(toolchain, "lib/runtime", file),
    ),
  ];
  const objects = join(target, "objects");
  mkdirSync(objects, { recursive: true });
  const temporary = `${output}.tmp`;
  const header = join(native, "private_runtime.h");
  const include = join(toolchain, "include");
  try {
    if (process.platform === "win32") {
      const cc = process.env.CC ?? Bun.which("cl") ?? Bun.which("clang-cl");
      if (!cc) throw new Error("Install an MSVC-compatible C toolchain and the Windows SDK");
      await run(
        [
          cc,
          "/nologo",
          "/LD",
          "/O2",
          "/std:c11",
          "/utf-8",
          `/I${include}`,
          `/FI${header}`,
          `/Fo${objects}\\`,
          ...sources,
          "/link",
          `/OUT:${temporary}`,
          "bcrypt.lib",
          "shell32.lib",
        ],
        native,
        env,
      );
    } else {
      const exports = join(target, "exports.map");
      writeFileSync(exports, "{ global: quickgui_extension_v1; local: *; };\n");
      await run(
        [
          process.env.CC ?? "cc",
          "-O2",
          "-std=c11",
          "-fPIC",
          "-fvisibility=hidden",
          "-fwrapv",
          "-fno-strict-aliasing",
          "-I",
          include,
          "-include",
          header,
          ...sources,
          "-lm",
          ...(process.platform === "darwin"
            ? ["-dynamiclib", "-Wl,-exported_symbol,_quickgui_extension_v1", "-Wl,-dead_strip"]
            : ["-shared", "-D_GNU_SOURCE", "-Wl,-z,defs", `-Wl,--version-script=${exports}`]),
          "-o",
          temporary,
        ],
        native,
        env,
      );
    }
    renameSync(temporary, output);
    copyFileSync(join(toolchain, "lib/core/LICENSE"), join(root, "artifacts/LICENSE-MoonBit"));
  } finally {
    rmSync(temporary, { force: true });
  }
}
