import type { AppearanceMode } from "@quickgui/native";
import type { TerminalPalette } from "@quickgui/solid";

export const GITHUB_LIGHT_TERMINAL = {
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
  ] as const satisfies TerminalPalette,
};

export const GITHUB_DARK_TERMINAL = {
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
  ] as const satisfies TerminalPalette,
};

export interface Theme {
  app: string;
  sidebar: string;
  surface: string;
  terminal: string;
  terminalText: string;
  terminalCursor: string;
  terminalPalette: TerminalPalette;
  hover: string;
  active: string;
  selected: string;
  selectedStrong: string;
  border: string;
  borderStrong: string;
  text: string;
  textSecondary: string;
  textTertiary: string;
  textGhost: string;
  accent: string;
  accentHover: string;
  accentWash: string;
  accentText: string;
  working: string;
  workingWash: string;
  success: string;
  successWash: string;
  warning: string;
  warningWash: string;
  danger: string;
  scrim: string;
  errorBackground: string;
}

export function themeFor(appearance: AppearanceMode): Theme {
  if (appearance === "light") {
    return {
      app: "#f7f7f6",
      sidebar: "#f1f1ef",
      surface: "#ffffff",
      terminal: GITHUB_LIGHT_TERMINAL.background,
      terminalText: GITHUB_LIGHT_TERMINAL.foreground,
      terminalCursor: GITHUB_LIGHT_TERMINAL.cursor,
      terminalPalette: GITHUB_LIGHT_TERMINAL.palette,
      hover: "#e9e9e6",
      active: "#dfdfdc",
      selected: "#e5e5e1",
      selectedStrong: "#dcdcd7",
      border: "#ddddda",
      borderStrong: "#cacac6",
      text: "#252524",
      textSecondary: "#62625f",
      textTertiary: "#858581",
      textGhost: "#a7a7a2",
      accent: "#cf684c",
      accentHover: "#bc5a41",
      accentWash: "#f5e7e2",
      accentText: "#ffffff",
      working: "#b27b20",
      workingWash: "#f4ebd9",
      success: "#2d9860",
      successWash: "#e1f1e8",
      warning: "#b27b20",
      warningWash: "#f4ebd9",
      danger: "#c94e43",
      scrim: "#18181655",
      errorBackground: "#f7e6e2",
    };
  }
  return {
    app: "#191918",
    sidebar: "#161615",
    surface: "#20201f",
    terminal: GITHUB_DARK_TERMINAL.background,
    terminalText: GITHUB_DARK_TERMINAL.foreground,
    terminalCursor: GITHUB_DARK_TERMINAL.cursor,
    terminalPalette: GITHUB_DARK_TERMINAL.palette,
    hover: "#242423",
    active: "#2b2b29",
    selected: "#282826",
    selectedStrong: "#30302d",
    border: "#2c2c2a",
    borderStrong: "#3c3c39",
    text: "#e4e4e1",
    textSecondary: "#aaa9a4",
    textTertiary: "#7e7d78",
    textGhost: "#595955",
    accent: "#e17b5d",
    accentHover: "#ec8a6d",
    accentWash: "#352520",
    accentText: "#1d110d",
    working: "#dfb25f",
    workingWash: "#332a1c",
    success: "#67c68a",
    successWash: "#1d3125",
    warning: "#dfb25f",
    warningWash: "#332a1c",
    danger: "#e06e66",
    scrim: "#050505bb",
    errorBackground: "#321d1b",
  };
}

export function createStyles(theme: Theme) {
  return {
    app: {
      position: "relative",
      display: "flex",
      flexDirection: "row",
      width: "100%",
      height: "100%",
      minWidth: 0,
      minHeight: 0,
      backgroundColor: theme.app,
      color: theme.text,
    },
    sidebar: {
      display: "flex",
      flexDirection: "column",
      flexShrink: 0,
      minWidth: 0,
      minHeight: 0,
      backgroundColor: theme.sidebar,
    },
    sidebarTitlebar: {
      display: "flex",
      height: 40,
      flexShrink: 0,
      alignItems: "center",
      paddingLeft: 78,
      paddingRight: 8,
      paddingTop: 2,
      appRegion: "drag",
    },
    titlebarIcon: {
      ...iconButton(theme, 24, 11),
      appRegion: "no-drag",
    },
    sidebarSection: {
      display: "flex",
      flexBasis: 0,
      minHeight: 0,
      flexDirection: "column",
      paddingLeft: 8,
      paddingRight: 8,
    },
    sidebarList: {
      display: "flex",
      flex: 1,
      minHeight: 0,
      flexDirection: "column",
      gap: 2,
      paddingBottom: 8,
      overflowY: "auto",
    },
    sidebarSectionDivider: {
      display: "flex",
      height: 7,
      flexShrink: 0,
      alignItems: "center",
      paddingLeft: 8,
      paddingRight: 8,
      cursor: "ns-resize",
      appRegion: "no-drag",
    },
    sidebarSectionDividerLine: {
      width: "100%",
      height: 1,
      backgroundColor: theme.border,
    },
    emptySidebarSection: {
      display: "flex",
      paddingLeft: 10,
      paddingRight: 10,
      paddingTop: 8,
      lineHeight: 16,
    },
    twoLineRow: {
      display: "flex",
      flex: 1,
      minWidth: 0,
      flexDirection: "column",
      justifyContent: "center",
      gap: 1,
    },
    rowLine: {
      display: "flex",
      minWidth: 0,
      alignItems: "center",
      gap: 5,
    },
    rowTitle: {
      color: theme.text,
      fontSize: 13.5,
      lineHeight: 16,
      fontWeight: 560,
      lineClamp: 1,
      textOverflow: "ellipsis",
    },
    rowCount: {
      color: theme.textGhost,
      fontSize: 11.5,
      lineHeight: 16,
    },
    agentTitle: {
      color: theme.text,
      fontSize: 13.5,
      lineHeight: 16,
      fontWeight: 560,
      lineClamp: 1,
      textOverflow: "ellipsis",
    },
    rowMeta: {
      color: theme.textTertiary,
      fontSize: 11.5,
      lineHeight: 15,
      lineClamp: 1,
      textOverflow: "ellipsis",
    },
    rowSeparator: {
      color: theme.textGhost,
      fontSize: 11.5,
      lineHeight: 15,
    },
    rowClose: {
      ...iconButton(theme, 20, 11),
      position: "absolute",
      top: 14,
      right: 5,
    },
    sidebarDivider: {
      position: "relative",
      display: "flex",
      width: 1,
      flexShrink: 0,
      alignItems: "center",
      justifyContent: "center",
      hitSlopLeft: 5,
      cursor: "ew-resize",
      appRegion: "no-drag",
    },
    sidebarDividerLine: {
      width: 1,
      height: "100%",
      backgroundColor: theme.border,
    },
    main: {
      display: "flex",
      flex: 1,
      minWidth: 0,
      minHeight: 0,
      flexDirection: "column",
      backgroundColor: theme.app,
    },
    tabBar: {
      display: "flex",
      height: 40,
      flexShrink: 0,
      alignItems: "center",
      gap: 4,
      paddingLeft: 8,
      paddingRight: 8,
      backgroundColor: theme.app,
      appRegion: "drag",
    },
    tabStrip: {
      display: "flex",
      minWidth: 0,
      alignItems: "center",
      gap: 3,
      overflow: "hidden",
      appRegion: "no-drag",
    },
    tabTitle: {
      color: theme.textSecondary,
      fontSize: 12.5,
      fontWeight: 560,
      lineClamp: 1,
      textOverflow: "ellipsis",
    },
    tabClose: {
      ...iconButton(theme, 20, 11),
      position: "absolute",
      top: 4,
      right: 3,
      appRegion: "no-drag",
    },
    tabAdd: {
      ...iconButton(theme, 24, 13),
      appRegion: "no-drag",
    },
    agentAction: {
      display: "flex",
      height: 24,
      flexShrink: 0,
      alignItems: "center",
      justifyContent: "center",
      gap: 5,
      paddingLeft: 9,
      paddingRight: 9,
      color: theme.textSecondary,
      backgroundColor: "transparent",
      hoverBackgroundColor: theme.hover,
      activeBackgroundColor: theme.active,
      transitionColors: 70,
      borderRadius: 5,
      fontSize: 11.5,
      fontWeight: 600,
      cursor: "default",
      appRegion: "no-drag",
    },
    toolbarSeparator: {
      width: 1,
      height: 16,
      flexShrink: 0,
      marginLeft: 2,
      marginRight: 2,
      backgroundColor: theme.borderStrong,
    },
    toolbarIcon: {
      ...iconButton(theme, 24, 12),
      appRegion: "no-drag",
    },
    hairline: { height: 1, flexShrink: 0, backgroundColor: theme.border },
    workspaceSurface: {
      position: "relative",
      display: "flex",
      flex: 1,
      minWidth: 0,
      minHeight: 0,
      backgroundColor: theme.app,
    },
    tabSurface: {
      position: "absolute",
      display: "flex",
      top: 0,
      right: 0,
      bottom: 0,
      left: 0,
      minWidth: 0,
      minHeight: 0,
      gap: 5,
    },
    terminalBody: {
      position: "relative",
      display: "flex",
      flex: 1,
      minWidth: 0,
      minHeight: 0,
      backgroundColor: theme.terminal,
    },
    emptyState: {
      display: "flex",
      flex: 1,
      minWidth: 0,
      minHeight: 0,
      alignItems: "center",
      justifyContent: "center",
      flexDirection: "column",
      gap: 9,
      backgroundColor: theme.terminal,
    },
    emptyCopy: { color: theme.textTertiary, fontSize: 13.5 },
    smallButton: {
      display: "flex",
      height: 26,
      alignItems: "center",
      justifyContent: "center",
      paddingLeft: 10,
      paddingRight: 10,
      backgroundColor: theme.surface,
      color: theme.textSecondary,
      borderColor: theme.borderStrong,
      borderWidth: 1,
      borderRadius: 5,
      hoverBackgroundColor: theme.hover,
      activeBackgroundColor: theme.active,
      fontSize: 11.5,
      cursor: "default",
    },
    errorBar: {
      display: "flex",
      minHeight: 30,
      flexShrink: 0,
      alignItems: "center",
      gap: 8,
      paddingLeft: 12,
      paddingRight: 8,
      backgroundColor: theme.errorBackground,
      borderColor: theme.borderStrong,
      borderWidth: 1,
    },
    errorAction: {
      display: "flex",
      height: 22,
      alignItems: "center",
      justifyContent: "center",
      paddingLeft: 8,
      paddingRight: 8,
      backgroundColor: "transparent",
      color: theme.danger,
      hoverBackgroundColor: theme.hover,
      activeBackgroundColor: theme.active,
      borderRadius: 4,
      fontSize: 10.5,
      cursor: "default",
    },
    modalScrim: {
      position: "absolute",
      display: "flex",
      top: 0,
      right: 0,
      bottom: 0,
      left: 0,
      alignItems: "center",
      justifyContent: "center",
      padding: 24,
      backgroundColor: theme.scrim,
    },
    agentSheet: {
      display: "flex",
      width: 472,
      flexDirection: "column",
      gap: 17,
      padding: 20,
      backgroundColor: theme.surface,
      borderColor: theme.borderStrong,
      borderWidth: 1,
      borderRadius: 9,
    },
    sheetHeader: { display: "flex", alignItems: "flex-start" },
    sheetClose: iconButton(theme, 26, 13),
    formGroup: { display: "flex", flexDirection: "column", gap: 7 },
    formLabel: {
      color: theme.textSecondary,
      fontSize: 11.5,
      fontWeight: 620,
    },
    launcherMark: {
      display: "flex",
      width: 24,
      height: 24,
      flexShrink: 0,
      alignItems: "center",
      justifyContent: "center",
      backgroundColor: theme.accentWash,
      borderRadius: 5,
    },
    prompt: {
      display: "flex",
      height: 88,
      paddingLeft: 10,
      paddingRight: 10,
      paddingTop: 9,
      paddingBottom: 9,
      backgroundColor: theme.terminal,
      color: theme.text,
      borderColor: theme.borderStrong,
      borderWidth: 1,
      borderRadius: 6,
      fontSize: 12.5,
    },
    catalogNotice: {
      display: "flex",
      minHeight: 36,
      alignItems: "center",
      paddingLeft: 10,
      paddingRight: 10,
      paddingTop: 7,
      paddingBottom: 7,
      backgroundColor: theme.accentWash,
      borderRadius: 5,
    },
    sheetFooter: { display: "flex", justifyContent: "flex-end", gap: 8 },
    toolbarSecondary: {
      display: "flex",
      height: 30,
      alignItems: "center",
      justifyContent: "center",
      paddingLeft: 10,
      paddingRight: 10,
      backgroundColor: "transparent",
      color: theme.textSecondary,
      borderColor: theme.border,
      borderWidth: 1,
      borderRadius: 5,
      hoverBackgroundColor: theme.hover,
      activeBackgroundColor: theme.active,
      fontSize: 11,
      cursor: "default",
    },
    toolbarPrimary: {
      display: "flex",
      height: 30,
      alignItems: "center",
      justifyContent: "center",
      paddingLeft: 11,
      paddingRight: 11,
      backgroundColor: theme.accent,
      color: theme.accentText,
      borderRadius: 5,
      hoverBackgroundColor: theme.accentHover,
      activeBackgroundColor: theme.accentHover,
      fontSize: 11,
      fontWeight: 680,
      cursor: "default",
    },
  } as const;
}

export type AppStyles = ReturnType<typeof createStyles>;

function iconButton(theme: Theme, size: number, fontSize: number) {
  return {
    display: "flex" as const,
    width: size,
    height: size,
    flexShrink: 0,
    alignItems: "center" as const,
    justifyContent: "center" as const,
    color: theme.textTertiary,
    backgroundColor: "transparent",
    hoverBackgroundColor: theme.hover,
    hoverColor: theme.text,
    activeBackgroundColor: theme.active,
    transitionColors: 70,
    borderRadius: 5,
    fontSize,
    cursor: "default",
  };
}

export function headingActionStyle(theme: Theme) {
  return { ...iconButton(theme, 20, 12), marginLeft: 5 };
}

export function sidebarRow(selected: boolean, theme: Theme) {
  return {
    display: "flex" as const,
    width: "100%",
    minWidth: 0,
    height: 48,
    flexShrink: 0,
    alignItems: "center" as const,
    gap: 8,
    paddingLeft: 8,
    paddingRight: selected ? 30 : 8,
    backgroundColor: selected ? theme.selected : "transparent",
    hoverBackgroundColor: selected ? theme.selectedStrong : theme.hover,
    activeBackgroundColor: theme.active,
    transitionColors: 70,
    borderRadius: 6,
    cursor: "default",
    userSelect: "none" as const,
  };
}

export function agentRow(selected: boolean, theme: Theme) {
  return {
    ...sidebarRow(selected, theme),
    height: 48,
    paddingRight: 8,
  };
}

export function tabItem(active: boolean, theme: Theme) {
  return {
    position: "relative" as const,
    display: "flex" as const,
    minWidth: 74,
    maxWidth: 170,
    height: 28,
    flexShrink: 1,
    alignItems: "center" as const,
    backgroundColor: active ? theme.selected : "transparent",
    borderRadius: 6,
    appRegion: "no-drag" as const,
  };
}

export function tabButton(active: boolean, theme: Theme) {
  return {
    display: "flex" as const,
    width: "100%",
    minWidth: 0,
    height: 28,
    alignItems: "center" as const,
    paddingLeft: 10,
    paddingRight: active ? 30 : 10,
    backgroundColor: "transparent",
    color: active ? theme.text : theme.textTertiary,
    hoverBackgroundColor: active ? theme.selectedStrong : theme.hover,
    activeBackgroundColor: theme.active,
    borderRadius: 6,
    cursor: "default",
  };
}

export function paneStyle(theme: Theme) {
  return {
    display: "flex" as const,
    flexGrow: 1,
    flexBasis: 0,
    minWidth: 0,
    minHeight: 0,
    flexDirection: "column" as const,
    backgroundColor: theme.terminal,
  };
}

export function launcherButton(
  selected: boolean,
  installed: boolean,
  theme: Theme,
) {
  return {
    display: "flex" as const,
    flex: 1,
    minWidth: 0,
    height: 54,
    alignItems: "center" as const,
    gap: 8,
    paddingLeft: 8,
    paddingRight: 8,
    backgroundColor: selected ? theme.selected : "transparent",
    color: installed ? theme.text : theme.textGhost,
    borderColor: selected ? theme.accent : theme.border,
    borderWidth: 1,
    borderRadius: 6,
    hoverBackgroundColor: installed ? theme.hover : "transparent",
    activeBackgroundColor: installed ? theme.active : "transparent",
    opacity: installed ? 1 : 0.5,
    cursor: "default",
  };
}
