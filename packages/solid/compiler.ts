import { transform } from "@solidjs/compiler";
import type { BunPlugin } from "bun";

export interface QuickGuiSolidPluginOptions {
  development?: boolean;
}

export function quickguiSolidPlugin(options: QuickGuiSolidPluginOptions = {}): BunPlugin {
  const development = options.development ?? process.env.NODE_ENV !== "production";
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
          dev: development,
        });
        return {
          contents: result.code,
          loader: "js",
        };
      });
    },
  };
}
