import data from "./generated/demo-sources.json";
import type { DemoSource } from "./demo-source";

export function getDemoSource(component: string): DemoSource | undefined {
  return (data as Record<string, DemoSource>)[component];
}
