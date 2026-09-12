import type { AppearanceMode } from "@quickgui/native";
import type { JSX } from "@quickgui/solid";

export interface Theme {
  appearance: AppearanceMode;
  /** Opaque content background. The sidebar is transparent over the window's vibrancy. */
  content: string;
  /** Translucent wash over the sidebar's vibrancy material, matching Finder's lighter sidebar. */
  sidebarWash: string;
  contentAlt: string;
  raised: string;
  text: string;
  textSecondary: string;
  textTertiary: string;
  textOnAccent: string;
  border: string;
  borderStrong: string;
  hover: string;
  active: string;
  /** Selected list row. */
  selection: string;
  selectionText: string;
  selectionMuted: string;
  accent: string;
  accentHover: string;
  accentWash: string;
  success: string;
  successWash: string;
  danger: string;
  dangerWash: string;
  warning: string;
  warningWash: string;
  /** Diff line backgrounds and gutters. */
  diffAdded: string;
  diffAddedGutter: string;
  diffAddedText: string;
  diffRemoved: string;
  diffRemovedGutter: string;
  diffRemovedText: string;
  diffHunk: string;
  diffHunkText: string;
  diffLineNumber: string;
  diffSelectedAdded: string;
  diffSelectedRemoved: string;
  diffSelectedContext: string;
  /** Commit graph lane colors. */
  graph: readonly string[];
  scrim: string;
  input: string;
  inputBorder: string;
  focusRing: string;
}

export function themeFor(appearance: AppearanceMode): Theme {
  if (appearance === "light") {
    return {
      appearance,
      content: "#ffffff",
      sidebarWash: "#00000000",
      contentAlt: "#f6f6f8",
      raised: "#ffffff",
      text: "#1d1d1f",
      textSecondary: "#5f5f66",
      textTertiary: "#9a9aa1",
      textOnAccent: "#ffffff",
      border: "#e4e4e8",
      borderStrong: "#cfcfd6",
      hover: "#00000009",
      active: "#00000014",
      selection: "#0000001c",
      selectionText: "#1d1d1f",
      selectionMuted: "#dbe6ff",
      accent: "#007aff",
      accentHover: "#0a6ff0",
      accentWash: "#e5f0ff",
      success: "#1a7f37",
      successWash: "#e6f6ea",
      danger: "#cf222e",
      dangerWash: "#ffebe9",
      warning: "#9a6700",
      warningWash: "#fff8c5",
      diffAdded: "#e8ffee",
      diffAddedGutter: "#c9f2d4",
      diffAddedText: "#1a7f37",
      diffRemoved: "#ffebe9",
      diffRemovedGutter: "#ffd0cc",
      diffRemovedText: "#cf222e",
      diffHunk: "#eef4ff",
      diffHunkText: "#3b6fd6",
      diffLineNumber: "#a1a1a8",
      diffSelectedAdded: "#bfe9cb",
      diffSelectedRemoved: "#f7c7c3",
      diffSelectedContext: "#dfe7fb",
      graph: [
        "#2f6bff",
        "#1a9e63",
        "#d0432f",
        "#b56cd8",
        "#e08a1e",
        "#1b9fb7",
        "#c74f9d",
        "#6d7fd6",
      ],
      scrim: "#1a1a2266",
      input: "#ffffff",
      inputBorder: "#00000026",
      focusRing: "#007aff66",
    };
  }
  return {
    appearance,
    content: "#1d1d1f",
    sidebarWash: "#00000000",
    contentAlt: "#232326",
    raised: "#2a2a2e",
    text: "#f2f2f5",
    textSecondary: "#a9a9b2",
    textTertiary: "#6f6f78",
    textOnAccent: "#ffffff",
    border: "#333338",
    borderStrong: "#45454c",
    hover: "#ffffff0d",
    active: "#ffffff1a",
    selection: "#ffffff29",
    selectionText: "#f2f2f5",
    selectionMuted: "#243657",
    accent: "#0a84ff",
    accentHover: "#2b93ff",
    accentWash: "#1c2d4a",
    success: "#4cc38a",
    successWash: "#16301f",
    danger: "#ff6b62",
    dangerWash: "#3a1d20",
    warning: "#e3b341",
    warningWash: "#3a2e12",
    diffAdded: "#15291c",
    diffAddedGutter: "#1f4a2d",
    diffAddedText: "#5fd48f",
    diffRemoved: "#33191c",
    diffRemovedGutter: "#5a2a2e",
    diffRemovedText: "#ff8a80",
    diffHunk: "#1a2231",
    diffHunkText: "#7aa7ff",
    diffLineNumber: "#5f5f68",
    diffSelectedAdded: "#215a35",
    diffSelectedRemoved: "#6b2f34",
    diffSelectedContext: "#2b3d63",
    graph: ["#4a86ff", "#3fbf7f", "#ff6b62", "#c792ea", "#f0a044", "#3ec1d3", "#e57bbd", "#8da2ff"],
    scrim: "#00000099",
    input: "#1a1a1c",
    inputBorder: "#ffffff26",
    focusRing: "#0a84ff66",
  };
}

export const UI_FONT_SIZE = 13;
export const LIST_FONT_SIZE = 13;
export const MONO_FONT_SIZE = 12;
export const ROW_HEIGHT = 28;
export const DIFF_ROW_HEIGHT = 20;
export const TITLEBAR_HEIGHT = 52;
/** Room above the sidebar's first row for a focus ring, which the scrolling container clips. */
export const SIDEBAR_TOP_INSET = 4;
export const TRANSITION = "bg 90ms, border-color 90ms, color 90ms, opacity 90ms";

type Style = JSX.Style;

/** Styles shared by every view; anything specific to one component lives with it. */
export function createStyles(theme: Theme) {
  const iconButton = (size: number): Style => ({
    display: "flex",
    width: size,
    height: size,
    flexShrink: 0,
    alignItems: "center",
    justifyContent: "center",
    color: theme.textSecondary,
    bg: "transparent",
    borderRadius: 5,
    cursor: "default",
    transition: TRANSITION,
    appRegion: "no-drag",
    hover: { bg: theme.hover, color: theme.text },
    active: { bg: theme.active },
    focus: { outline: `2px solid ${theme.focusRing}` },
    disabled: { opacity: 0.4 },
  });
  const button = (kind: "primary" | "secondary" | "danger"): Style => ({
    display: "flex",
    flexDirection: "row",
    height: 22,
    flexShrink: 0,
    alignItems: "center",
    justifyContent: "center",
    gap: 5,
    paddingLeft: 11,
    paddingRight: 11,
    borderRadius: 6,
    fontSize: UI_FONT_SIZE,
    cursor: "default",
    userSelect: "none",
    appRegion: "no-drag",
    disabled: { opacity: 0.4 },
    ...(kind === "primary"
      ? {
          bg: theme.accent,
          color: theme.textOnAccent,
          hover: { bg: theme.accentHover },
        }
      : {
          bg: theme.raised,
          color: kind === "danger" ? theme.danger : theme.text,
          borderWidth: 1,
          borderColor: theme.borderStrong,
          hover: { bg: theme.hover },
        }),
  });
  return {
    app: {
      position: "relative",
      display: "flex",
      flexDirection: "row",
      width: "100%",
      height: "100%",
      minWidth: 0,
      minHeight: 0,
      bg: "transparent",
      color: theme.text,
      fontSize: UI_FONT_SIZE,
    } satisfies Style,
    sidebar: {
      display: "flex",
      flexDirection: "column",
      height: "100%",
      minWidth: 0,
      minHeight: 0,
      bg: "transparent",
    } satisfies Style,
    sidebarTitlebar: {
      display: "flex",
      height: TITLEBAR_HEIGHT,
      flexShrink: 0,
      alignItems: "center",
      paddingLeft: 84,
      paddingRight: 10,
      appRegion: "drag",
    } satisfies Style,
    sidebarScroll: {
      display: "flex",
      flex: 1,
      minHeight: 0,
      flexDirection: "column",
      gap: 2,
      paddingTop: SIDEBAR_TOP_INSET,
      paddingLeft: 10,
      paddingRight: 10,
      paddingBottom: 12,
      overflowY: "auto",
    } satisfies Style,
    sectionLabel: {
      display: "flex",
      height: 24,
      flexShrink: 0,
      alignItems: "center",
      paddingLeft: 8,
      paddingRight: 4,
      marginTop: 10,
      gap: 4,
    } satisfies Style,
    sectionLabelText: {
      flex: 1,
      minWidth: 0,
      color: theme.textTertiary,
      fontSize: 11,
      fontWeight: 700,
      letterSpacing: 0.3,
      textTransform: "uppercase",
      lineClamp: 1,
      textOverflow: "ellipsis",
    } satisfies Style,
    listPanel: {
      display: "flex",
      flex: 1,
      minHeight: 0,
      flexDirection: "column",
      overflowY: "auto",
      padding: 12,
      gap: 4,
    } satisfies Style,
    listHeader: {
      display: "flex",
      flexDirection: "row",
      alignItems: "center",
      height: 32,
      flexShrink: 0,
      gap: 8,
    } satisfies Style,
    listTitle: {
      fontSize: 11,
      fontWeight: 700,
      textTransform: "uppercase",
      color: theme.textTertiary,
      flex: 1,
    } satisfies Style,
    main: {
      display: "flex",
      flex: 1,
      minWidth: 0,
      minHeight: 0,
      flexDirection: "column",
      bg: theme.content,
    } satisfies Style,
    toolbar: {
      display: "flex",
      height: TITLEBAR_HEIGHT,
      flexShrink: 0,
      alignItems: "center",
      gap: 6,
      paddingLeft: 14,
      paddingRight: 12,
      borderBottomWidth: 1,
      borderColor: theme.border,
      bg: theme.content,
      appRegion: "drag",
    } satisfies Style,
    hairline: { height: 1, flexShrink: 0, bg: theme.border } satisfies Style,
    vhairline: { width: 1, flexShrink: 0, bg: theme.border } satisfies Style,
    iconButton,
    button,
    tinyButton: {
      ...button("secondary"),
      height: 18,
      fontSize: 11,
      paddingLeft: 8,
      paddingRight: 8,
    } satisfies Style,
    input: {
      display: "flex",
      height: 22,
      paddingLeft: 7,
      paddingRight: 7,
      bg: theme.input,
      color: theme.text,
      borderWidth: 1,
      borderColor: theme.inputBorder,
      borderRadius: 6,
      fontSize: UI_FONT_SIZE,
      appRegion: "no-drag",
      transition: "border-color 90ms",
      focus: { borderColor: theme.accent },
      invalid: { borderColor: theme.danger },
    } satisfies Style,
    textArea: {
      display: "flex",
      width: "100%",
      minWidth: 0,
      minHeight: 72,
      paddingLeft: 7,
      paddingRight: 7,
      paddingTop: 4,
      paddingBottom: 4,
      bg: theme.input,
      color: theme.text,
      borderWidth: 1,
      borderColor: theme.inputBorder,
      borderRadius: 6,
      fontSize: UI_FONT_SIZE,
      lineHeight: 18,
      appRegion: "no-drag",
      transition: "border-color 90ms",
      focus: { borderColor: theme.accent },
    } satisfies Style,
    badge: {
      display: "flex",
      height: 18,
      minWidth: 18,
      flexShrink: 0,
      alignItems: "center",
      justifyContent: "center",
      paddingLeft: 6,
      paddingRight: 6,
      borderRadius: 9,
      bg: theme.hover,
      color: theme.textSecondary,
      fontSize: 11,
      fontWeight: 700,
    } satisfies Style,
    popup: {
      display: "flex",
      flexDirection: "column",
      bg: theme.raised,
      borderWidth: 1,
      borderColor: theme.borderStrong,
      borderRadius: 8,
      boxShadow:
        theme.appearance === "light"
          ? "0 12px 32px -8px #00000040, 0 2px 6px #00000014"
          : "0 12px 32px -8px #000000aa, 0 2px 6px #00000060",
    } satisfies Style,
    dialogPortal: {
      position: "absolute",
      top: 0,
      right: 0,
      bottom: 0,
      left: 0,
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
    } satisfies Style,
    dialogBackdrop: {
      position: "absolute",
      top: 0,
      right: 0,
      bottom: 0,
      left: 0,
      bg: theme.scrim,
    } satisfies Style,
    dialogPopup: {
      display: "flex",
      flexDirection: "column",
      width: 440,
      gap: 14,
      padding: 20,
      bg: theme.raised,
      borderWidth: 1,
      borderColor: theme.borderStrong,
      borderRadius: 10,
      boxShadow: "0 24px 60px -12px #00000066",
    } satisfies Style,
    dialogTitle: { fontSize: 15, fontWeight: 700, color: theme.text } satisfies Style,
    dialogDescription: {
      fontSize: 12.5,
      lineHeight: 18,
      color: theme.textSecondary,
    } satisfies Style,
    fieldLabel: { fontSize: 12, fontWeight: 600, color: theme.textSecondary } satisfies Style,
    mono: { fontFamily: "monospace", fontSize: MONO_FONT_SIZE } satisfies Style,
    emptyState: {
      display: "flex",
      flex: 1,
      minHeight: 0,
      flexDirection: "column",
      alignItems: "center",
      justifyContent: "center",
      gap: 8,
      padding: 32,
    } satisfies Style,
    emptyTitle: {
      fontSize: 15,
      fontWeight: 500,
      color: theme.textTertiary,
      textAlign: "center",
    } satisfies Style,
    emptyCopy: {
      fontSize: 12,
      lineHeight: 17,
      color: theme.textTertiary,
      textAlign: "center",
      maxWidth: 360,
    } satisfies Style,
  };
}

export type Styles = ReturnType<typeof createStyles>;

/** Color for a status letter. */
export function statusColor(theme: Theme, code: string): string {
  switch (code) {
    case "A":
    case "?":
      return theme.success;
    case "D":
      return theme.danger;
    case "R":
    case "C":
      return theme.accent;
    case "U":
      return theme.warning;
    case "T":
      return theme.warning;
    default:
      return theme.warning;
  }
}
