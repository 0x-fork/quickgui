import { expect, test } from "bun:test";
import { mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { resolveConfig } from "../config.ts";
import { packageLinux, packageWindows } from "./pipeline.ts";

test("AppDir and Debian payloads include exactly the selected extension libraries", async () => {
  const root = mkdtempSync(join(tmpdir(), "quickgui-linux-extensions-"));
  try {
    const config = resolveConfig(
      {
        name: "Demo",
        identifier: "test.demo",
        linux: { appImage: true, deb: true, maintainer: "Test <test@example.com>" },
      },
      root,
    );
    const executablePath = join(root, config.executableName);
    const libraries = ["libquickgui_host.so", "libquickgui_terminal.so"].map((name) =>
      join(root, name),
    );
    for (const file of [...libraries, executablePath, join(root, "libquickgui_unused.so")])
      writeFileSync(file, "payload");
    await packageLinux({
      config,
      target: "linux-x64",
      executablePath,
      stagingRoot: root,
      libraries,
      run: async () => {},
    });
    expect(readdirSync(join(root, `${config.executableName}.AppDir/usr/bin`)).sort()).toEqual(
      [config.executableName, "libquickgui_host.so", "libquickgui_terminal.so"].sort(),
    );
    const deb = readFileSync(
      join(
        root,
        readdirSync(root).find((name) => name.endsWith(".deb"))!,
      ),
    );
    let data: Buffer | undefined;
    for (let offset = 8; offset < deb.length;) {
      const name = deb
        .toString("ascii", offset, offset + 16)
        .trim()
        .replace(/\/$/, "");
      const size = Number(deb.toString("ascii", offset + 48, offset + 58).trim());
      if (name === "data.tar.gz")
        data = Buffer.from(Bun.gunzipSync(deb.subarray(offset + 60, offset + 60 + size)));
      offset += 60 + size + (size % 2);
    }
    expect(data).toBeDefined();
    expect(data!.includes(Buffer.from("libquickgui_host.so"))).toBe(true);
    expect(data!.includes(Buffer.from("libquickgui_terminal.so"))).toBe(true);
    expect(data!.includes(Buffer.from("libquickgui_unused.so"))).toBe(false);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("Windows installer installs and uninstalls each selected library", async () => {
  const root = mkdtempSync(join(tmpdir(), "quickgui-windows-extensions-"));
  try {
    const config = resolveConfig({ name: "Demo", identifier: "test.demo" }, root);
    const libraries = ["quickgui_host.dll", "quickgui_terminal.dll"].map((name) =>
      join(root, name),
    );
    const executablePath = join(root, "Demo.exe");
    for (const file of [...libraries, executablePath]) writeFileSync(file, "payload");
    await packageWindows({
      config,
      executablePath,
      stagingRoot: root,
      libraries,
      run: async () => {
        writeFileSync(
          join(root, `${config.executableName}-${config.version}-setup.exe`),
          "installer",
        );
      },
    });
    const script = readFileSync(join(root, `${config.executableName}.nsi`), "utf8");
    for (const name of ["quickgui_host.dll", "quickgui_terminal.dll"]) {
      expect(script).toContain(`File "/oname=${name}"`);
      expect(script).toContain(`Delete "$INSTDIR\\${name}"`);
    }
    expect(script).not.toContain("quickgui_unused");
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
