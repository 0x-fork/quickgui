import { expect, test } from "bun:test";
import { resolveConfig } from "./config.ts";
import { goBuildPlan, sharedLibraryName } from "./native-build.ts";

test("Go builds disable CGO and keep the Rust library out of the link command", () => {
  const config = resolveConfig({ name: "Go App", identifier: "dev.test.go", native: { tags: ["demo"] } }, "/tmp/go-app");
  const plan = goBuildPlan({ config, mode: "development", target: "darwin-arm64", executablePath: "/tmp/My App", fonts: ["fonts/Test.ttf"] });
  expect(plan.argv.slice(0, 2)).toEqual(["go", "build"]);
  expect(plan.argv.at(-1)).toBe(".");
  expect(plan.env.CGO_ENABLED).toBe("0");
  expect(plan.env.GOOS).toBe("darwin");
  expect(plan.env.GOARCH).toBe("arm64");
  expect(plan.argv).toContain("demo");
  const flags = plan.argv[plan.argv.indexOf("-ldflags") + 1]!;
  const metadata = JSON.parse(Buffer.from(flags.split("buildMetadata=")[1]!, "base64url").toString());
  expect(metadata.identifier).toBe("dev.test.go.dev");
  expect(metadata.fonts).toEqual(["fonts/Test.ttf"]);
  expect(plan.argv.join(" ")).not.toContain("cargo");
  expect(plan.argv.join(" ")).not.toContain("clang");
});

test("production Go builds strip symbols and map x64 to amd64", () => {
  const config = resolveConfig({ name: "Go App", identifier: "dev.test.go", entry: "cmd/app" }, "/tmp/go-app");
  const plan = goBuildPlan({ config, mode: "production", target: "darwin-x64", executablePath: "/tmp/app", fonts: [] });
  expect(plan.argv).toContain("-trimpath");
  expect(plan.argv.at(-1)).toBe("./cmd/app");
  expect(plan.env.GOARCH).toBe("amd64");
  expect(sharedLibraryName("darwin-x64")).toBe("libquickgui_host.dylib");
});
