import { expect, test } from "bun:test";
import {
  isSettled,
  idleMeasurement,
  memorySample,
  processRole,
  type MemoryProcess,
  type MemorySample,
} from "./benchmark-memory.ts";

const appPath = "/bench/Orbit.app";

function process(
  pid: number,
  role: MemoryProcess["role"],
  footprintBytes: number,
  cpuTimeNs = 0,
): MemoryProcess {
  return {
    pid,
    role,
    footprintBytes,
    cpuTimeNs,
    rssBytes: footprintBytes * 2,
    name: String(pid),
    path: `${appPath}/${pid}`,
  };
}

function history(seconds: number, at: (second: number) => MemoryProcess[]): MemorySample[] {
  const samples: MemorySample[] = [];
  for (let second = 0; second <= seconds; second++)
    samples.push(memorySample(second * 1000, at(second), samples.at(-1)));
  return samples;
}

test("counts app and rendering helpers while excluding AutoFill and macOS services", () => {
  expect(processRole(`${appPath}/Contents/MacOS/Orbit`, 1, 1, appPath)).toBe("app");
  expect(
    processRole(
      `${appPath}/Contents/Frameworks/Orbit Helper.app/Contents/MacOS/Orbit Helper`,
      2,
      1,
      appPath,
    ),
  ).toBe("framework-helper");
  for (const name of ["WebContent", "GPU", "Networking"])
    expect(
      processRole(
        `/System/Library/Frameworks/WebKit.framework/XPCServices/com.apple.WebKit.${name}`,
        2,
        1,
        appPath,
      ),
    ).toBe("framework-helper");
  for (const path of [
    "/System/Library/com.apple.SafariPlatformSupport.Helper",
    "/System/Library/SetStoreUpdateService",
    "/System/Library/com.apple.audio.SandboxHelper",
    `${appPath}-other/Contents/MacOS/Orbit`,
  ])
    expect(processRole(path, 3, 1, appPath)).toBe("os-service");
  const sample = memorySample(0, [
    process(1, "app", 80_000_000),
    process(2, "framework-helper", 40_000_000),
    process(3, "os-service", 16_400_000),
  ]);
  expect(sample.footprintBytes).toBe(120_000_000);
  expect(sample.mainProcessBytes).toBe(80_000_000);
  expect(sample.frameworkHelperBytes).toBe(40_000_000);
  expect(sample.excludedOsServiceBytes).toBe(16_400_000);
  expect(sample.processes).toHaveLength(3);
});

test("requires a real idle window after at least thirty seconds", () => {
  const at = () => [process(1, "app", 80_000_000)];
  expect(isSettled(history(5, at))).toBe(false);
  expect(isSettled(history(29, at))).toBe(false);
  expect(isSettled(history(30, at))).toBe(true);
  expect(isSettled([memorySample(30_000, at())])).toBe(false);
});

test("a late startup memory release must settle before sampling", () => {
  const at = (second: number) => [process(1, "app", second < 25 ? 84_000_000 : 80_000_000)];
  expect(isSettled(history(30, at))).toBe(false);
  expect(isSettled(history(40, at))).toBe(true);
});

test("checks CPU deltas across the app and renderers, ignoring AutoFill activity", () => {
  const at = (second: number) => [
    process(1, "app", 80_000_000, second * 1_000_000),
    process(2, "os-service", second * 5_000_000, second * 500_000_000),
  ];
  const idle = history(30, at);
  expect(idle.at(-1)!.cpuPercent).toBeCloseTo(0.1);
  expect(isSettled(idle)).toBe(true);
  const busy = history(30, (second) => [
    ...at(second),
    process(3, "framework-helper", 30_000_000, second * 20_000_000),
  ]);
  expect(busy.at(-1)!.cpuPercent).toBeCloseTo(2.1);
  expect(isSettled(busy)).toBe(false);
});

test("helper restarts and active memory growth invalidate the idle plateau", () => {
  expect(
    isSettled(
      history(30, (s) => [
        process(1, "app", 80_000_000),
        process(s < 25 ? 2 : 3, "framework-helper", 30_000_000),
      ]),
    ),
  ).toBe(false);
  expect(isSettled(history(30, (s) => [process(1, "app", 80_000_000 + s * 100_000)]))).toBe(false);
});

test("a late release restarts sampling until a complete idle interval is observed", () => {
  const at = (s: number) => [process(1, "app", s < 35 ? 84_000_000 : 80_000_000)];
  expect(idleMeasurement(history(30, at))).toBeNull();
  expect(idleMeasurement(history(40, at))).toBeNull();
  expect(idleMeasurement(history(59, at))).toBeNull();
  const measured = idleMeasurement(history(60, at))!;
  expect(measured.idleDetectedAtMs).toBe(50_000);
  expect(measured.samples).toHaveLength(10);
  expect(measured.samples.every((s) => s.footprintBytes === 80_000_000)).toBe(true);
  expect(measured.startupSamples.some((s) => s.footprintBytes === 84_000_000)).toBe(true);
});
