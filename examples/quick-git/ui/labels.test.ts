import { describe, expect, test } from "bun:test";

import { repositoryLabels } from "./labels.ts";

describe("repository labels", () => {
  test("uses the folder name and qualifies duplicates with their parent", () => {
    const labels = repositoryLabels(["/Users/me/work/app", "/Users/me/personal/app", "/Users/me/tools"]);
    expect(labels.get("/Users/me/work/app")).toBe("work/app");
    expect(labels.get("/Users/me/personal/app")).toBe("personal/app");
    expect(labels.get("/Users/me/tools")).toBe("tools");
  });
});
