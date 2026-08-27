import { transform } from "@solidjs/compiler";
import type { BunPlugin } from "bun";

export function quickguiSolidPlugin(): BunPlugin {
  return {
    name: "quickgui-solid",
    setup(build) {
      build.onLoad({ filter: /\.[jt]sx$/ }, async ({ path }) => {
        const source = await Bun.file(path).text();
        const result = transform(source, {
          filename: path,
          moduleName: "@quickgui/solid",
          generate: "universal",
          hydratable: false,
          sourceMap: true,
          dev: process.env.NODE_ENV !== "production",
        });
        return {
          contents: result.code,
          loader: "js",
        };
      });
    },
  };
}
