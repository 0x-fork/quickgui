import { describe, expect, test } from "bun:test";

import type { GraphRow } from "../git/log.ts";
import { LANE_WIDTH, laneX, paintGraphRow } from "./graph.ts";

function row(overrides: Partial<GraphRow>): GraphRow {
  return {
    lane: 0,
    color: 0,
    incoming: true,
    joins: [],
    passing: [],
    edges: [{ fromLane: 0, toLane: 0, color: 0 }],
    laneCount: 1,
    merge: false,
    ...overrides,
  };
}

describe("graph painting", () => {
  test("paints a straight lane through the node with the node's color only", () => {
    const layers = paintGraphRow(row({}), 40, 26);
    expect(layers).toHaveLength(1);
    expect(layers[0]!.color).toBe(0);
    // In from the top to the node, then out of the node to the bottom, at the lane's center.
    expect(layers[0]!.source).toContain(`d="M${laneX(0)} 0V13M${laneX(0)} 13V26"`);
    expect(layers[0]!.source).toContain(`<circle cx="${laneX(0)}" cy="13" r="4"`);
    expect(layers[0]!.source).toContain('viewBox="0 0 40 26"');
  });

  test("a tip has no line above its node and a root none below", () => {
    const tip = paintGraphRow(row({ incoming: false }), 40, 26);
    expect(tip[0]!.source).toContain(`d="M${laneX(0)} 13V26"`);
    const root = paintGraphRow(row({ edges: [] }), 40, 26);
    expect(root[0]!.source).toContain(`d="M${laneX(0)} 0V13"`);
  });

  test("groups passing lanes and elbows by color and turns elbows the right way", () => {
    const layers = paintGraphRow(
      row({
        passing: [{ lane: 1, color: 1 }],
        edges: [
          { fromLane: 0, toLane: 0, color: 0 },
          { fromLane: 0, toLane: 2, color: 2 },
        ],
        joins: [{ fromLane: 3, color: 3 }],
        laneCount: 4,
      }),
      70,
      26,
    );
    const byColor = new Map(layers.map((layer) => [layer.color, layer.source]));
    expect([...byColor.keys()].sort()).toEqual([0, 1, 2, 3]);
    // The passing lane is a full-height line.
    expect(byColor.get(1)).toContain(`d="M${laneX(1)} 0V26"`);
    // Leaving the node to the right: level, then a clockwise quarter turn down into lane 2.
    const radius = LANE_WIDTH / 2;
    expect(byColor.get(2)).toContain(
      `M${laneX(0)} 13H${laneX(2) - radius}A${radius} ${radius} 0 0 1 ${laneX(2)} ${13 + radius}V26`,
    );
    // Arriving from lane 3 on the right: down, a clockwise quarter turn, then level into the node.
    expect(byColor.get(3)).toContain(
      `M${laneX(3)} 0V${13 - radius}A${radius} ${radius} 0 0 1 ${laneX(3) - radius} 13H${laneX(0)}`,
    );
    // Only the node's color paints the disc.
    expect(byColor.get(0)).toContain("<circle");
    expect(byColor.get(2)).not.toContain("<circle");
  });

  test("elbows toward the left turn counterclockwise", () => {
    const layers = paintGraphRow(
      row({
        lane: 2,
        color: 2,
        edges: [
          { fromLane: 2, toLane: 2, color: 2 },
          { fromLane: 2, toLane: 0, color: 0 },
        ],
        joins: [{ fromLane: 1, color: 1 }],
        laneCount: 3,
      }),
      60,
      26,
    );
    const byColor = new Map(layers.map((layer) => [layer.color, layer.source]));
    const radius = LANE_WIDTH / 2;
    expect(byColor.get(0)).toContain(
      `M${laneX(2)} 13H${laneX(0) + radius}A${radius} ${radius} 0 0 0 ${laneX(0)} ${13 + radius}V26`,
    );
    expect(byColor.get(1)).toContain(
      `M${laneX(1)} 0V${13 - radius}A${radius} ${radius} 0 0 0 ${laneX(1) + radius} 13H${laneX(2)}`,
    );
  });

  test("keeps the elbow inside short rows", () => {
    const layers = paintGraphRow(row({ edges: [{ fromLane: 0, toLane: 1, color: 1 }] }), 40, 10);
    // Half a row is 5, so the elbow radius shrinks to 4 to keep the turn inside the row.
    expect(layers.find((layer) => layer.color === 1)!.source).toContain("A4 4 0 0 1");
  });
});
