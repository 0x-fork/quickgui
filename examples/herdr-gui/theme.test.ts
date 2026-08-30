import { describe, expect, test } from "bun:test";

import {
  GITHUB_DARK_TERMINAL,
  GITHUB_LIGHT_TERMINAL,
  agentRow,
  createStyles,
  sidebarRow,
  themeFor,
} from "./theme.ts";

describe("GitHub terminal themes", () => {
  test("uses the authored GitHub Light Default terminal colors", () => {
    expect(GITHUB_LIGHT_TERMINAL).toEqual({
      background: "#ffffff",
      foreground: "#1f2328",
      cursor: "#0969da",
      palette: [
        "#24292f",
        "#cf222e",
        "#116329",
        "#4d2d00",
        "#0969da",
        "#8250df",
        "#1b7c83",
        "#6e7781",
        "#57606a",
        "#a40e26",
        "#1a7f37",
        "#633c01",
        "#218bff",
        "#a475f9",
        "#3192aa",
        "#8c959f",
      ],
    });
  });

  test("uses the authored GitHub Dark Default terminal colors", () => {
    expect(GITHUB_DARK_TERMINAL).toEqual({
      background: "#0d1117",
      foreground: "#e6edf3",
      cursor: "#2f81f7",
      palette: [
        "#484f58",
        "#ff7b72",
        "#3fb950",
        "#d29922",
        "#58a6ff",
        "#bc8cff",
        "#39c5cf",
        "#b1bac4",
        "#6e7681",
        "#ffa198",
        "#56d364",
        "#e3b341",
        "#79c0ff",
        "#d2a8ff",
        "#56d4dd",
        "#ffffff",
      ],
    });
    expect(themeFor("dark").terminalPalette).toBe(
      GITHUB_DARK_TERMINAL.palette,
    );
  });
});

describe("sidebar row layout", () => {
  test("keeps space and agent rows fixed-height in vertical lists", () => {
    const theme = themeFor("light");

    for (const row of [sidebarRow(false, theme), agentRow(false, theme)]) {
      expect(row).toMatchObject({
        width: "100%",
        height: 48,
        flexShrink: 0,
      });
      expect(row).not.toHaveProperty("flex");
    }
  });

  test("keeps two-line sidebar content compact and aligned", () => {
    const styles = createStyles(themeFor("light"));

    expect(styles.twoLineRow.gap).toBe(1);
    expect(styles.rowTitle.lineHeight).toBe(16);
    expect(styles.agentTitle.lineHeight).toBe(16);
    expect(styles.rowMeta.lineHeight).toBe(15);
    expect(styles.rowCount.lineHeight).toBe(16);
  });
});
