import generated from "./generated/component-api.json";
import type { ComponentApi } from "./component-api";
import type { ComponentDocKind } from "./component-docs";
import type { DocsFrontend } from "./docs";

export function getComponentApi(
  frontend: DocsFrontend,
  kind: ComponentDocKind,
  slug: string,
): ComponentApi {
  const api = (generated as Record<string, ComponentApi>)[`${frontend}/${kind}/${slug}`];
  if (!api) throw new Error(`Missing component API: ${frontend}/${kind}/${slug}`);
  return api;
}
