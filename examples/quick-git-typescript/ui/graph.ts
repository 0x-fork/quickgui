/**
 * Painting of one commit-graph row as SVG.
 *
 * The framework rasterizes an `Svg` element into a one-channel mask tinted by its `color`, so a
 * row is painted as one document per lane color: continuous vertical lines for lanes that pass
 * through, the node's own line coming in from above, quarter-circle elbows where a line leaves
 * the node for another lane or arrives from one, and a filled disc for the commit. Adjacent rows
 * paint every lane at the same x and up to their shared edge, so the lines read as continuous.
 */

import type { GraphRow } from "../git/log.ts";

export const LANE_WIDTH = 14;
/** Horizontal inset before the first lane. */
export const GRAPH_INSET = 6;
export const LINE_WIDTH = 2;
export const NODE_RADIUS = 4;

/** Center x of a lane inside the graph cell. */
export function laneX(lane: number): number {
  return GRAPH_INSET + lane * LANE_WIDTH + LANE_WIDTH / 2;
}

/** One SVG document painting every segment of one color in a row. */
export interface GraphLayer {
  /** Lane color index into the theme palette. */
  color: number;
  source: string;
}

/**
 * SVG documents that paint `row` in a cell of `width` × `height` logical pixels, one per color.
 *
 * Elbows are quarter circles whose radius is half a lane, so a line leaving the node runs level
 * for the rest of the lane and turns down exactly at its new lane; a line arriving from above does
 * the same in mirror. Straight segments use butt caps so a lane continues seamlessly into the
 * neighbouring rows.
 */
export function paintGraphRow(row: GraphRow, width: number, height: number): GraphLayer[] {
  const middle = height / 2;
  const radius = Math.min(LANE_WIDTH / 2, Math.max(1, middle - 1));
  const segments = new Map<number, string[]>();
  const add = (color: number, path: string): void => {
    const list = segments.get(color);
    if (list) list.push(path);
    else segments.set(color, [path]);
  };
  const nodeX = laneX(row.lane);
  for (const lane of row.passing) add(lane.color, `M${laneX(lane.lane)} 0V${height}`);
  if (row.incoming) add(row.color, `M${nodeX} 0V${middle}`);
  for (const join of row.joins) {
    const fromX = laneX(join.fromLane);
    if (fromX === nodeX) {
      add(join.color, `M${nodeX} 0V${middle}`);
      continue;
    }
    // Travel direction toward the node decides which way the elbow turns: arriving from the
    // right turns clockwise on screen (sweep 1), arriving from the left counterclockwise.
    const direction = fromX > nodeX ? -1 : 1;
    const sweep = direction < 0 ? 1 : 0;
    add(
      join.color,
      `M${fromX} 0V${middle - radius}A${radius} ${radius} 0 0 ${sweep} ${fromX + direction * radius} ${middle}H${nodeX}`,
    );
  }
  for (const edge of row.edges) {
    const toX = laneX(edge.toLane);
    if (toX === nodeX) {
      add(edge.color, `M${nodeX} ${middle}V${height}`);
      continue;
    }
    const direction = toX > nodeX ? 1 : -1;
    const sweep = direction > 0 ? 1 : 0;
    add(
      edge.color,
      `M${nodeX} ${middle}H${toX - direction * radius}A${radius} ${radius} 0 0 ${sweep} ${toX} ${middle + radius}V${height}`,
    );
  }
  if (!segments.has(row.color)) segments.set(row.color, []);

  const layers: GraphLayer[] = [];
  for (const [color, paths] of segments) {
    const stroke =
      paths.length > 0
        ? `<path d="${paths.join("")}" fill="none" stroke="#000" stroke-width="${LINE_WIDTH}"/>`
        : "";
    const node =
      color === row.color
        ? `<circle cx="${nodeX}" cy="${middle}" r="${NODE_RADIUS}" fill="#000"/>`
        : "";
    layers.push({
      color,
      source: `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">${stroke}${node}</svg>`,
    });
  }
  return layers;
}
