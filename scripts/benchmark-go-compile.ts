#!/usr/bin/env bun
/** Measure Go compilation in temporary SDK copies without touching the user's build cache. */
import { createHash } from "node:crypto";
import {
  cpSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { arch, cpus, platform, release, tmpdir, totalmem } from "node:os";
import { dirname, join, resolve } from "node:path";
import { parseArgs } from "node:util";

const { values } = parseArgs({
  args: process.argv.slice(2),
  options: {
    baseline: { type: "string", default: "HEAD" },
    samples: { type: "string", default: "9" },
    "cold-samples": { type: "string", default: "3" },
    output: { type: "string", default: "benchmarks/go-compile/results.json" },
    keep: { type: "boolean", default: false },
  },
});
const samples = Number(values.samples),
  coldSamples = Number(values["cold-samples"]);
if (![samples, coldSamples].every((value) => Number.isInteger(value) && value > 0))
  throw new Error("Sample counts must be positive integers");
const root = resolve(import.meta.dir, "..");
const work = mkdtempSync(join(tmpdir(), "quickgui-go-compile-"));
const output = resolve(root, values.output!);
const warmCache = join(work, "warm-cache");
const baseEnv = {
  ...process.env,
  CGO_ENABLED: "0",
  GOTOOLCHAIN: "local",
  GOWORK: "off",
  GOFLAGS: "",
  GOPROXY: "off",
  GOCACHEPROG: "",
};
const flags = ["-trimpath", "-mod=readonly", "-buildvcs=false"];

async function command(argv: string[], cwd = root, env: NodeJS.ProcessEnv = baseEnv) {
  const start = performance.now();
  const child = Bun.spawn(argv, {
    cwd,
    env,
    stdin: "ignore",
    stdout: "pipe",
    stderr: "pipe",
  });
  const [status, stdout, stderr] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);
  const elapsedMs = performance.now() - start;
  if (status !== 0) throw new Error(`${argv.join(" ")} failed in ${cwd}:\n${stdout}${stderr}`);
  return { elapsedMs, stdout, stderr };
}
function files(path: string): string[] {
  return readdirSync(path, { withFileTypes: true })
    .flatMap((entry) => {
      if ([".git", ".quickgui", "node_modules", "_build"].includes(entry.name)) return [];
      const full = join(path, entry.name);
      return entry.isDirectory() ? files(full) : [full];
    })
    .sort();
}
function sourceStats(sdk: string) {
  const sources = files(join(sdk, "ui")).filter(
    (path) => path.endsWith(".go") && !path.endsWith("_test.go"),
  );
  const hash = createHash("sha256");
  let bytes = 0,
    lines = 0;
  for (const path of sources) {
    const text = readFileSync(path, "utf8");
    hash.update(path.slice(sdk.length));
    hash.update(text);
    bytes += Buffer.byteLength(text);
    lines += text.split("\n").length - 1;
  }
  return { files: sources.length, bytes, lines, sha256: hash.digest("hex") };
}
function trimGeneratedMethods(sdk: string) {
  const target = join(sdk, "ui/element_generated.go");
  const source = readFileSync(target, "utf8");
  const referenced = new Set<string>();
  for (const path of files(sdk)) {
    if (path === target || !path.endsWith(".go") || path.endsWith("_test.go")) continue;
    for (const match of readFileSync(path, "utf8").matchAll(/\.(\w+)\s*\(/g))
      referenced.add(match[1]!);
  }
  const removed: string[] = [],
    kept: string[] = [];
  const result = source.replace(
    /^\/\/ [^\n]*\nfunc \(element \*Element\) (\w+)[^\n]*\n\treturn [^\n]*\n\}\n*/gm,
    (declaration, name: string) => {
      if (referenced.has(name)) {
        kept.push(name);
        return declaration;
      }
      removed.push(name);
      return "";
    },
  );
  if (
    removed.length === 0 ||
    removed.length + kept.length !== [...source.matchAll(/^func \(element \*Element\)/gm)].length
  )
    throw new Error("Generated method declarations changed; update the pruning parser");
  writeFileSync(target, result);
  return {
    removed,
    kept,
    bytesBefore: Buffer.byteLength(source),
    bytesAfter: Buffer.byteLength(result),
  };
}
function summarize(durations: number[]) {
  const sorted = [...durations].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  const medianMs =
    sorted.length % 2 ? sorted[middle]! : (sorted[middle - 1]! + sorted[middle]!) / 2;
  return { medianMs, minMs: sorted[0]!, maxMs: sorted.at(-1)!, samplesMs: durations };
}
type Variant = {
  name: string;
  sdk: string;
  app: string;
  stats: ReturnType<typeof sourceStats>;
  timings: Record<string, number[]>;
};
function editMarker(directory: string, label: string) {
  writeFileSync(
    join(directory, "compile_benchmark_revision.go"),
    `package ${directory.endsWith("ui") ? "ui" : "main"}\nconst compileBenchmarkRevision = ${JSON.stringify(label)}\n`,
  );
}
async function build(variant: Variant, cache: string, target: "sdk" | "app", trace = false) {
  const cwd = variant.app;
  const argv = [
    "go",
    "build",
    ...flags,
    ...(trace ? ["-x"] : []),
    ...(target === "sdk"
      ? ["github.com/egoist/quickgui/go/ui"]
      : ["-o", join(variant.app, "counter"), "."]),
  ];
  return command(argv, cwd, { ...baseEnv, GOCACHE: cache });
}
function compiledPackages(trace: string) {
  return [...trace.matchAll(/\/compile\s+[^\n]*?\s-p\s+(\S+)/g)].map((match) => match[1]!);
}

try {
  const baseline = (await command(["git", "rev-parse", values.baseline!])).stdout.trim();
  const fixture = (
    await command(["git", "show", `${baseline}:examples/counter/main.go`])
  ).stdout.replace('"QuickGUI Counter"', '"QuickGUI Counter " + compileBenchmarkRevision');
  const fixtureSum = createHash("sha256").update(fixture).digest("hex");
  const fixtureMod = (
    await command(["git", "show", `${baseline}:examples/counter/go.mod`])
  ).stdout.replace("=> ../../go", "=> ../sdk");
  const fixtureGoSum = (await command(["git", "show", `${baseline}:examples/counter/go.sum`]))
    .stdout;
  const beforeDirectory = join(work, "before");
  mkdirSync(beforeDirectory, { recursive: true });
  const archive = join(work, "baseline.tar");
  await command(["git", "archive", "--format=tar", `--output=${archive}`, baseline, "go"]);
  await command(["tar", "-xf", archive, "-C", beforeDirectory]);
  cpSync(join(beforeDirectory, "go"), join(beforeDirectory, "sdk"), { recursive: true });
  cpSync(join(root, "go"), join(work, "current/sdk"), { recursive: true });
  cpSync(join(work, "current/sdk"), join(work, "pruned/sdk"), { recursive: true });
  const pruning = trimGeneratedMethods(join(work, "pruned/sdk"));
  const variants: Variant[] = ["before", "current", "pruned"].map((name) => {
    const sdk = join(work, name, "sdk"),
      app = join(work, name, "app");
    mkdirSync(app, { recursive: true });
    writeFileSync(join(app, "main.go"), fixture);
    writeFileSync(join(app, "go.mod"), fixtureMod);
    writeFileSync(join(app, "go.sum"), fixtureGoSum);
    editMarker(app, "initial");
    return {
      name,
      sdk,
      app,
      stats: sourceStats(sdk),
      timings: { sdkRecompile: [], appEdit: [], emptyGoCache: [] },
    };
  });
  console.log(
    `Baseline ${baseline}; pruned ${pruning.removed.length}/${pruning.removed.length + pruning.kept.length} generated methods`,
  );
  console.log("Priming the private cache and checking each variant...");
  for (const variant of variants) {
    await build(variant, warmCache, "app");
    console.log(`Ready: ${variant.name}`);
  }

  const traces: Record<string, { sdk: string[]; app: string[]; appLinks: boolean }> = {};
  for (const variant of variants) {
    editMarker(join(variant.sdk, "ui"), `sdk-trace-${variant.name}`);
    const sdkTrace = await build(variant, warmCache, "sdk", true);
    editMarker(variant.app, `app-trace-${variant.name}`);
    const appTrace = await build(variant, warmCache, "app", true);
    const sdk = compiledPackages(sdkTrace.stderr),
      app = compiledPackages(appTrace.stderr);
    const appLinks = /\/link\s/.test(appTrace.stderr);
    if (sdk.length !== 1 || sdk[0] !== "github.com/egoist/quickgui/go/ui")
      throw new Error(`SDK rebuild compiled unexpected packages: ${sdk}`);
    if (app.length !== 1 || app[0] !== "main" || !appLinks)
      throw new Error(`App edit missed the SDK cache or linker: ${app}`);
    traces[variant.name] = { sdk, app, appLinks };
  }

  // Run one build at a time. Rotating variant order balances warmup/thermal drift.
  for (const phase of ["sdkRecompile", "appEdit"] as const) {
    for (let round = 0; round < samples; round++) {
      for (let index = 0; index < variants.length; index++) {
        const variant = variants[(round + index) % variants.length]!;
        editMarker(
          phase === "sdkRecompile" ? join(variant.sdk, "ui") : variant.app,
          `${phase}-${variant.name}-${round}`,
        );
        const measurement = await build(
          variant,
          warmCache,
          phase === "sdkRecompile" ? "sdk" : "app",
        );
        variant.timings[phase]!.push(measurement.elapsedMs);
      }
      console.log(`${phase}: ${round + 1}/${samples} paired rounds`);
    }
  }
  for (let round = 0; round < coldSamples; round++) {
    for (let index = 0; index < variants.length; index++) {
      const variant = variants[(round + index) % variants.length]!;
      const cache = join(work, `cold-${round}-${variant.name}`);
      mkdirSync(cache);
      rmSync(join(variant.app, "counter"), { force: true });
      const measurement = await build(variant, cache, "app");
      variant.timings.emptyGoCache!.push(measurement.elapsedMs);
      console.log(
        `emptyGoCache: ${variant.name} ${round + 1}/${coldSamples}: ${(measurement.elapsedMs / 1000).toFixed(3)} s`,
      );
      rmSync(cache, { recursive: true, force: true });
    }
  }
  // Trace a separate cold build outside the timed samples to check that the
  // standard library really recompiles, rather than trusting a cache label.
  const coldTraceVariant = variants.find((variant) => variant.name === "current")!;
  rmSync(join(coldTraceVariant.app, "counter"), { force: true });
  const coldTrace = await build(coldTraceVariant, join(work, "cold-trace"), "app", true);
  const coldPackages = compiledPackages(coldTrace.stderr);
  const coldLinks = /\/link\s/.test(coldTrace.stderr);
  for (const required of ["runtime", "reflect", "github.com/egoist/quickgui/go/ui", "main"]) {
    if (!coldPackages.includes(required)) {
      throw new Error(`Cold-cache trace did not rebuild ${required}: ${coldPackages}`);
    }
  }
  if (!coldLinks) throw new Error("Cold-cache trace did not link a new output binary");
  const results = {
    measuredAt: new Date().toISOString(),
    baseline,
    environment: {
      go: (await command(["go", "version"])).stdout.trim(),
      cpu: cpus()[0]?.model,
      logicalCpus: cpus().length,
      memoryBytes: totalmem(),
      platform: platform(),
      architecture: arch(),
      osRelease: release(),
      cgoEnabled: false,
      flags,
    },
    methodology: {
      fixture:
        "Identical baseline Counter source in every variant, with a revision constant appended to its window title",
      fixtureSha256: fixtureSum,
      timing: "Wall clock, sequential builds, rotating variant order; medians",
      sdkRecompile:
        "Change a private constant in ui; dependencies cached; build the ui import path from the app module without linking an app",
      appEdit:
        "Change a window-title revision constant in main; SDK cached; compile and link the Counter app",
      emptyGoCache:
        "Fresh private GOCACHE and removed output binary for each sample, including stdlib and link; module download and OS file caches are not cleared",
      network:
        "GOPROXY=off; installed module cache reused; no Rust compilation, code generation, application launch, or shared cache clearing",
    },
    pruning,
    traces,
    coldVerification: { variant: "current", packages: coldPackages, appLinks: coldLinks },
    variants: variants.map(({ name, stats, timings }) => ({
      name,
      stats,
      timings: Object.fromEntries(
        Object.entries(timings).map(([phase, durations]) => [phase, summarize(durations)]),
      ),
    })),
  };
  mkdirSync(dirname(output), { recursive: true });
  writeFileSync(output, JSON.stringify(results, null, 2) + "\n");
  console.table(
    results.variants.map((variant) => ({
      variant: variant.name,
      sdkMs: variant.timings.sdkRecompile!.medianMs.toFixed(1),
      appEditMs: variant.timings.appEdit!.medianMs.toFixed(1),
      coldSeconds: (variant.timings.emptyGoCache!.medianMs / 1000).toFixed(3),
    })),
  );
  console.log(`Saved ${output}`);
} finally {
  if (values.keep) console.log(`Kept temporary workspace ${work}`);
  else rmSync(work, { recursive: true, force: true });
}
