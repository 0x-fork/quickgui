import data from "./generated/demo-sources.json";
import type { DemoSource } from "./demo-source";
import type { DocsFrontend } from "./docs";

export function getDemoSource(frontend: DocsFrontend, component: string): DemoSource | undefined {
  return (data as Record<string, DemoSource>)[`${frontend}/${component}`];
}
