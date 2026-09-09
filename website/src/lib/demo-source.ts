import type { ComponentApi } from "./component-api";

export interface DemoSource {
  path: string;
  language: ComponentApi["language"];
  code: string;
  html: string;
}
