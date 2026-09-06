/**
 * The gallery's palette and shared styles.
 *
 * The window reports which appearance it is really rendering in. Nothing here hardcodes a
 * palette: `light` is the default, `dark` is used only while the system really is dark, and the
 * signal is written from the window's appearance event, never from a component body.
 */

import { createSignal, type Style } from "@quickgui/ui";
import type { MenuAppearance } from "@quickgui/ui/menu";
import type { PickerAppearance } from "@quickgui/ui/select";

export interface Palette {
  window: string;
  sidebar: string;
  panel: string;
  panelAlt: string;
  control: string;
  controlHover: string;
  controlActive: string;
  ink: string;
  muted: string;
  faint: string;
  border: string;
  accent: string;
  /** The accent a filled control shows while hovered, so its text stays readable. */
  accentHover: string;
  onAccent: string;
  selection: string;
  popup: string;
  backdrop: string;
  track: string;
  danger: string;
  dangerHover: string;
}

export const lightPalette: Palette = {
  window: "#f4f5f7",
  sidebar: "#ebedf1",
  panel: "#ffffff",
  panelAlt: "#f8f9fb",
  control: "#ffffff",
  controlHover: "#eef1f6",
  controlActive: "#e2e8f4",
  ink: "#131820",
  muted: "#5a6472",
  faint: "#8b94a3",
  border: "#d9dde4",
  accent: "#2563eb",
  accentHover: "#1d4fd7",
  onAccent: "#ffffff",
  selection: "#dbe6fd",
  popup: "#ffffff",
  backdrop: "#1e293b66",
  track: "#e4e7ec",
  danger: "#b42318",
  dangerHover: "#9a1d14",
};

export const darkPalette: Palette = {
  window: "#0e1117",
  sidebar: "#131924",
  panel: "#171e2a",
  panelAlt: "#1b2331",
  control: "#1c2432",
  controlHover: "#263042",
  controlActive: "#2f3a50",
  ink: "#e7edf7",
  muted: "#98a4b6",
  faint: "#6e7a8c",
  border: "#2a3446",
  accent: "#5b93f7",
  accentHover: "#7aa7ff",
  onAccent: "#08111f",
  selection: "#1f2d47",
  popup: "#171e2a",
  backdrop: "#010409aa",
  track: "#252f41",
  danger: "#f0736a",
  dangerHover: "#f58c84",
};

export const [appearance, setAppearance] = createSignal<string>("light");

/** The window extent the core last reported, written from the window's resize event. */
export const [viewportWidth, setViewportWidth] = createSignal<number>(1080);
export const [viewportHeight, setViewportHeight] = createSignal<number>(780);

/** The palette the window's current appearance selects. */
export function p(): Palette {
  return appearance() === "dark" ? darkPalette : lightPalette;
}

export function controlStyle(): Style {
  return {
    display: "flex",
    flexDirection: "row",
    alignItems: "center",
    justifyContent: "center",
    gap: 6,
    height: 30,
    paddingLeft: 12,
    paddingRight: 12,
    borderRadius: 8,
    backgroundColor: p().control,
    borderColor: p().border,
    borderWidth: 1,
    color: p().ink,
    fontSize: 13,
    cursor: "default",
    userSelect: "none",
    hover: { backgroundColor: p().controlHover },
    focus: { outline: "2px solid " + p().accent },
    outlineOffset: 2,
  };
}

export function inputStyle(): Style {
  return {
    height: 30,
    paddingLeft: 10,
    paddingRight: 10,
    borderRadius: 8,
    backgroundColor: p().panelAlt,
    borderColor: p().border,
    borderWidth: 1,
    color: p().ink,
    fontSize: 13,
    focus: { outline: "2px solid " + p().accent },
    outlineOffset: 2,
  };
}

export function popupStyle(): Style {
  return {
    display: "flex",
    flexDirection: "column",
    gap: 8,
    padding: 14,
    borderRadius: 12,
    backgroundColor: p().popup,
    borderColor: p().border,
    borderWidth: 1,
    boxShadow: "0 18px 40px " + (appearance() === "dark" ? "#00000088" : "#0f172a2e"),
  };
}

export function menuRowStyle(): Style {
  return {
    display: "flex",
    flexDirection: "row",
    alignItems: "center",
    justifyContent: "space-between",
    gap: 12,
    paddingLeft: 10,
    paddingRight: 10,
    height: 28,
    borderRadius: 6,
    hover: { backgroundColor: p().selection },
  };
}

/** Fill the declaring element's box. */
export function overlayFill(): Style {
  return { position: "absolute", top: 0, right: 0, bottom: 0, left: 0 };
}

export function pickerAppearance(): PickerAppearance {
  return {
    width: 240,
    rowHeight: 28,
    radius: 10,
    background: p().popup,
    color: p().ink,
    highlightBackground: p().accent,
    highlightColor: p().onAccent,
    selectedBackground: p().selection,
    mutedColor: p().muted,
  };
}

export function menuAppearance(): MenuAppearance {
  return {
    width: 220,
    radius: 10,
    background: p().popup,
    color: p().ink,
    highlightBackground: p().accent,
    highlightColor: p().onAccent,
    mutedColor: p().muted,
  };
}

/** Gauges and sliders declare an explicit track width so the core's arithmetic has a basis. */
export const GAUGE_WIDTH = 420;
