import { transform } from "@solidjs/compiler";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";
import type { BunPlugin } from "bun";

/** Adapted from QuickGUI's former Bun Worker frontend; Solid 2 must use its client build. */
export function quickguiSolidPlugin(
  options: { projectRoot?: string; development?: boolean } = {},
): BunPlugin {
  const root = options.projectRoot ?? process.cwd();
  const manifest = Bun.resolveSync("solid-js/package.json", root);
  const development = options.development ?? process.env.NODE_ENV !== "production";
  const runtime = join(dirname(manifest), "dist", development ? "solid.dev.js" : "solid.js");
  return {
    name: "quickgui-solid-2",
    setup(build) {
      // Merely adding the browser condition is insufficient when node appears first in exports.
      if (build.config) build.onResolve({ filter: /^solid-js$/ }, () => ({ path: runtime }));
      else
        build.module("solid-js", async () => ({
          exports: await import(pathToFileURL(runtime).href),
          loader: "object",
        }));
      build.onLoad({ filter: /\.[jt]sx$/ }, async ({ path }) => {
        const result = transform(await Bun.file(path).text(), {
          filename: path,
          moduleName: "@quickgui/solid",
          generate: "universal",
          hydratable: false,
          sourceMap: true,
          dev: development,
        });
        return {
          contents:
            result.code +
            (result.map
              ? `\n//# sourceMappingURL=data:application/json;base64,${Buffer.from(result.map).toString("base64")}\n`
              : ""),
          loader: "ts",
        };
      });
    },
  };
}
