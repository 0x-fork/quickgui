import { describe, expect, test } from "bun:test";

import { focusedCloseTarget, type WorkspaceTab } from "./model.ts";

const tab: WorkspaceTab = {
  id: 7,
  spaceId: "/project",
  number: 1,
  paneIds: [11, 12],
  activePaneId: 12,
  splitDirection: "horizontal",
};

describe("focusedCloseTarget", () => {
  test("closes the focused pane while the active tab is split", () => {
    expect(focusedCloseTarget(tab, 12, 3)).toEqual({ kind: "pane", id: 12 });
  });

  test("closes the active tab after its last pane", () => {
    expect(
      focusedCloseTarget({ ...tab, paneIds: [12] }, 12, 3),
    ).toEqual({ kind: "tab", id: 7 });
  });

  test("closes the window only for the final unsplit tab", () => {
    expect(
      focusedCloseTarget({ ...tab, paneIds: [12] }, 12, 1),
    ).toEqual({ kind: "window" });
  });
});
