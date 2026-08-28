import { transform } from "@solidjs/compiler";
import type { BunPlugin } from "bun";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

export interface QuickGuiSolidPluginOptions {
  development?: boolean;
  projectRoot?: string;
}

export function quickguiSolidPlugin(options: QuickGuiSolidPluginOptions = {}): BunPlugin {
  const development = options.development ?? process.env.NODE_ENV !== "production";
  const projectRoot = options.projectRoot ?? process.cwd();
  const solidPackage = resolveSolidPackage(projectRoot);
  const solidRuntime = join(dirname(solidPackage), "dist", development ? "dev.js" : "solid.js");
  const solidRuntimeUrl = pathToFileURL(solidRuntime).href;
  return {
    name: "quickgui-solid",
    setup(build) {
      // Bun's default `node` condition selects Solid's server build. QuickGUI is a persistent UI
      // runtime, so signal setters must always resolve to the client/reactive implementation even
      // when a development app imports its source dynamically from outside the compiled host.
      if (build.config) {
        build.onResolve({ filter: /^solid-js$/ }, () => ({ path: solidRuntime }));
      } else {
        // Runtime plugins cannot intercept a bare package without `.` or `:` via `onResolve`.
        // Registering the exact module specifier also covers @solidjs/universal's JS import.
        build.module("solid-js", async () => ({
          exports: await import(solidRuntimeUrl),
          loader: "object",
        }));
      }
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
          // The Solid transform lowers JSX but intentionally preserves TypeScript syntax such as
          // interfaces and type-only imports. Let Bun perform the final TypeScript erasure.
          loader: "ts",
        };
      });
    },
  };
}

function resolveSolidPackage(projectRoot: string): string {
  try {
    return Bun.resolveSync("solid-js/package.json", projectRoot);
  } catch {
    return Bun.resolveSync("solid-js/package.json", dirname(fileURLToPath(import.meta.url)));
  }
}
