/** Declaration shapes shared by the generated interfaces, the runtime setters, and the compiler. */

/** One gradient stop; a bare color string spaces evenly with its neighbours. */
export interface GradientStop {
  color: string;
  position?: number;
}

/** The declared object form of a gradient. At most eight stops are retained by the core. */
export interface GradientDeclaration {
  type: "linear" | "radial" | "conic";
  angle?: number;
  fromAngle?: number;
  shape?: "circle" | "ellipse";
  extent?: "closest-side" | "farthest-side" | "farthest-corner";
  center?: { x: number; y: number };
  interpolation?: "linear-srgb" | "srgb" | "oklab";
  stops: GradientStop[];
}

export type TransitionEasing = "linear" | "ease" | "ease-in" | "ease-out" | "ease-in-out";

export interface TransitionDeclaration {
  /** One or more paint properties: `all`, `background-color`, `border-color`, `border-width`, `border-radius`, `color`, `box-shadow`, `opacity`. */
  property?: string | string[];
  /** Alias of `property`. */
  properties?: string[];
  duration?: number | string;
  easing?: TransitionEasing;
  /** Alias of `easing`. */
  timingFunction?: TransitionEasing;
  maxFps?: number;
}

/** A CSS `matrix(a, b, c, d, tx, ty)` in the core's own component order. */
export interface TransformMatrix {
  a: number;
  b: number;
  c: number;
  d: number;
  tx: number;
  ty: number;
}

export interface TextShadowDeclaration {
  offsetX: number;
  offsetY: number;
  blur?: number;
  color?: string;
}

/**
 * The paint-only overrides one interaction state swaps in: exactly what the core's own
 * `ElementStateStyle` carries. Layout never changes with a state.
 */
export interface StateStyle {
  background?: number | string | GradientDeclaration;
  backgroundColor?: number | string;
  color?: number | string;
  borderColor?: number | string;
  borderWidth?: number | string;
  borderRadius?: number | string;
  outline?: number | string;
  boxShadow?: string;
  opacity?: number;
  cursor?: string;
  transform?: string | string[] | TransformMatrix;
  transformOrigin?: string;
}

/** One `groupHover` or `groupActive` entry; it may name the group it follows. */
export interface GroupStateStyle extends StateStyle {
  group?: string;
}

export interface Extent {
  width: number;
  height: number;
}

export interface StringMapEntry {
  name: string;
  value: string;
}

/** Environment variables and other string maps travel as ordered pairs. */
export type StringMap = StringMapEntry[];

/** Sixteen ANSI colors: black through white, then the eight bright variants. */
export type TerminalPalette = (number | string)[];

export interface KeymapBinding {
  /** Accelerator such as `cmd+s`. */
  keys: string;
  /** Binding id dispatched through `onAction`. */
  action: string;
}

export type Keymap = KeymapBinding[];

export interface DragFile {
  path: string;
  directory?: boolean;
}

export interface DragSource {
  id?: string;
  text?: string;
  url?: string;
  files?: DragFile[];
}

export type DropKind = "local" | "files";

/** One ordered item of a toolbar or toggle group. */
export interface ComponentItem {
  value: string;
  disabled?: boolean;
}

export interface OptionEntry {
  value: string;
  label: string;
  group?: string;
  disabled?: boolean;
}

export type OptionSource = OptionEntry[];

export interface ComponentAppearance {
  width?: number;
  itemHeight?: number;
  fontSize?: number;
  radius?: number;
  padding?: number;
  background?: number | string;
  color?: number | string;
  highlightBackground?: number | string;
  highlightColor?: number | string;
  mutedColor?: number | string;
}

export interface TableColumn {
  id: string;
  title: string;
  width?: number;
  minWidth?: number;
  maxWidth?: number;
  align?: "start" | "center" | "end";
  sortable?: boolean;
  resizable?: boolean;
  editable?: boolean;
}

export interface TreeNode {
  id: string;
  label: string;
  children?: TreeNode[];
  hasChildren?: boolean;
  disabled?: boolean;
}

export interface TreeNodeChildren {
  parent: string;
  children: TreeNode[];
}

export interface TableEditing {
  row: number;
  column: number;
  value: string;
}

export interface ToastDeclaration {
  id: string;
  title?: string;
  description?: string;
  kind?: string;
  timeout?: number;
}
