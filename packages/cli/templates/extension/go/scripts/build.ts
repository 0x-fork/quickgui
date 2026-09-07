import { resolve } from "node:path";

const child = Bun.spawn(["go", "build", "./..."], {
  cwd: resolve(import.meta.dir, ".."),
  env: { ...process.env, CGO_ENABLED: "0" },
  stdin: "inherit",
  stdout: "inherit",
  stderr: "inherit",
});
process.exitCode = await child.exited;
