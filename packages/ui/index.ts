/**
 * `@quickgui/ui`: fine-grained reactivity plus the host element components for QuickGUI
 * applications compiled to native code.
 *
 * JSX in application code is lowered by the QuickGUI compiler to the functions in `runtime.ts`;
 * the element components here also work when called directly.
 */

import { NativeNodeTag, type NativeNode, type QuickGuiEvent } from "@quickgui/native";

import { applyProps, type NativeProps } from "./generated.ts";
import { element } from "./runtime.ts";

export * from "./reactive.ts";
export * from "./runtime.ts";
export * from "./types.ts";
export * from "./generated.ts";
export type { NativeNode, QuickGuiEvent } from "@quickgui/native";

/** Unstyled block/flex/grid container. */
export function View(props: NativeProps): NativeNode {
  const node = element(NativeNodeTag.View);
  applyProps(node, props);
  return node;
}

/** Unstyled text-semantic container whose string children remain individually reactive. */
export function Text(props: NativeProps): NativeNode {
  const node = element(NativeNodeTag.View);
  applyProps(node, props);
  return node;
}

/** Unstyled, focusable native button with web-style arrow-cursor behavior by default. */
export function Button(props: NativeProps): NativeNode {
  const node = element(NativeNodeTag.Button);
  applyProps(node, props);
  return node;
}

/** Controlled, unstyled single-line native text input. */
export function Input(props: NativeProps): NativeNode {
  const node = element(NativeNodeTag.Input);
  applyProps(node, props);
  return node;
}

/** Controlled, unstyled multiline native text area. */
export function TextArea(props: NativeProps): NativeNode {
  const node = element(NativeNodeTag.Input);
  applyProps(node, { ...props, multiline: true });
  return node;
}

/** Retained, incremental native Markdown document. */
export function Markdown(props: NativeProps): NativeNode {
  const node = element(NativeNodeTag.Markdown);
  applyProps(node, props);
  return node;
}

/** Unstyled variable-height list; only visible child blocks are mounted by QuickGUI core. */
export function VirtualList(props: NativeProps): NativeNode {
  const node = element(NativeNodeTag.VirtualList);
  applyProps(node, props);
  return node;
}

/** Real PTY terminal rendered by QuickGUI core through libghostty-vt. */
export function Terminal(props: NativeProps): NativeNode {
  const node = element(NativeNodeTag.Terminal);
  applyProps(node, props);
  return node;
}

/** Parsed-once retained SVG mask tinted by the inherited `color` style. */
export function Svg(props: NativeProps): NativeNode {
  const node = element(NativeNodeTag.Svg);
  applyProps(node, props);
  return node;
}

/** Bounded raster image decoded once per declared source. */
export function Image(props: NativeProps): NativeNode {
  const node = element(NativeNodeTag.Image);
  applyProps(node, props);
  return node;
}

/** Retained custom WGSL shader surface. */
export function Shader(props: NativeProps): NativeNode {
  const node = element(NativeNodeTag.Shader);
  applyProps(node, props);
  return node;
}

// ---------------------------------------------------------------------------
// Typed event payloads
// ---------------------------------------------------------------------------

export interface InputModifiers {
  shift: boolean;
  control: boolean;
  alt: boolean;
  meta: boolean;
}

export interface KeyEventDetails extends InputModifiers {
  /** DOM-shaped normalized key name, such as `"a"`, `"ArrowUp"`, or `"Escape"`. */
  key: string;
  /** Composed text the platform reported for this press, when it produced any. */
  text?: string;
  repeat?: boolean;
}

export interface MouseEventDetails extends InputModifiers {
  x: number;
  y: number;
  button?: string;
  pressedButton?: string;
  /** Exact native multi-click count. The first press is `1`. */
  clickCount?: number;
  firstMouse?: boolean;
}

export interface WheelEventDetails extends InputModifiers {
  x: number;
  y: number;
  deltaX: number;
  deltaY: number;
  /** `true` for a trackpad or precise wheel. */
  precise: boolean;
  phase: string;
}

export interface GestureEventDetails extends InputModifiers {
  x: number;
  y: number;
  delta?: number;
  phase?: string;
  pressure?: number;
  stage?: string;
}

export interface DropEventDetails extends InputModifiers {
  x: number;
  y: number;
  id?: string;
  source?: number;
  paths?: string[];
  origin?: string;
}

export interface PointerPoint {
  x: number;
  y: number;
}

export interface CapturedPointerEvent {
  /** `down`, `move`, `up`, or `cancel`. */
  phase: string;
  position: PointerPoint;
  origin: PointerPoint;
  localPosition: PointerPoint;
  localOrigin: PointerPoint;
  delta: PointerPoint;
  button: string;
}

export interface TerminalStatusEvent {
  /** `starting`, `running`, `exited`, or `failed`. */
  status: string;
  title: string;
  workingDirectory?: string;
  processId?: number;
  exitCode?: number;
  signal?: string;
  message?: string;
  agent?: string;
  agentStatus?: string;
  agentProcessId?: number;
}

export interface MenuSelectDetails {
  id: string;
  checked?: boolean;
  submenu?: boolean;
}

export interface VisibleRange {
  start: number;
  end: number;
}

export interface TableSortState {
  column: string;
  direction: string;
}

export interface TableCell {
  row: number;
  column: number;
}

export interface ColumnWidth {
  id: string;
  width: number;
}

/** The active tab's laid-out box, published by the core during paint. */
export interface TabsIndicatorGeometry {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** Which triggers the core's declared validation mode answers. */
export interface FieldValidationTriggers {
  change: boolean;
  blur: boolean;
  submit: boolean;
}

/** How long the core waits before validating on each trigger, in milliseconds. */
export interface FieldValidationDelays {
  change: number | null;
  blur: number | null;
  submit: number | null;
}

/** Which triggers validate a field, and how long the core waits before each. */
export interface FieldValidationDetails {
  triggers: FieldValidationTriggers;
  delay: FieldValidationDelays;
}

/** The end of one inline table edit, reported with the core's own commit decision. */
export interface TableEditEndDetails {
  row: number;
  column: number;
  committed: boolean;
}

/** One toast's place in the core's own stack. */
export interface ToastStackEntry {
  id: string;
  index: number;
  type: string;
  /** Older than the viewport's `limit`, and styled back rather than silenced. */
  limited: boolean;
  expanded: boolean;
  swiping: boolean;
  /** Live swipe displacement in logical pixels, for the application to translate by. */
  swipeMovement: number;
  /** `index * pitch`, computed by the core from the declared stack pitch. */
  offset: number;
}

/** A scroll area's clamped offset. */
export interface ScrollOffset {
  x: number;
  y: number;
}

/** The core's own select or combobox part state, styled from the way Base UI styles `data-*`. */
export interface PickerPartState {
  popupOpen?: boolean;
  popupSide?: string;
  pressed?: boolean;
  placeholder?: boolean;
  valid?: boolean;
  invalid?: boolean;
  dirty?: boolean;
  touched?: boolean;
  filled?: boolean;
  focused?: boolean;
  readOnly?: boolean;
  required?: boolean;
  /** The polite live-region text the core derived for `Combobox.Status`. */
  status?: string;
  /** Whether the query really matched nothing, which is when `Combobox.Empty` mounts. */
  empty?: boolean;
  resultCount?: number;
}

/**
 * The payload of a native `componentchange` event.
 *
 * Every key is optional and typed exactly the way the Rust host emits it: the native JSON
 * decoder rejects a `null` where no `null` is declared, so a key the host can clear is typed
 * with `null` and a key that changes shape between components has one name per shape.
 */
export interface ComponentChangeDetails {
  /** The side and alignment an anchored surface really resolved to. */
  placement?: AnchorPlacementDetails;
  /** The side a tab or navigation selection travelled toward; `null` for no direction. */
  activationDirection?: string | null;
  /** The active tab's box, when an indicator declared a placement. */
  indicator?: TabsIndicatorGeometry | null;
  /** A dialog finished its declared enter or exit transition. */
  openChangeComplete?: boolean;
  /** Checked values of a checkbox group, in the declared order. */
  checkedValues?: string[];
  /** Which triggers the core's declared validation mode answers. */
  validation?: FieldValidationTriggers;
  /** How long the core waits before validating on each trigger. */
  validationDelay?: FieldValidationDelays;
  /** A gauge's derived status: `progressing`, `complete`, or `indeterminate`. */
  status?: string;
  /** The value a gauge or slider formatted through its declared `format`. */
  displayValue?: string | null;
  /** How far a gauge's value has travelled, from 0 to 1. */
  completion?: number | null;
  /** Whether a slider drag is in flight. */
  dragging?: boolean;
  /** Whether a slider or number-field change is the core's own commit boundary. */
  committed?: boolean;
  /** Slider thumb values, in ascending thumb order. */
  values?: number[];
  /** Splitter pane sizes in logical pixels. */
  sizes?: number[];
  /** The toolbar item that now owns the single roving Tab stop. */
  active?: string | null;
  /** The pressed toggle-group values, in declared item order. */
  pressed?: string[];
  /** Selected option, tree node, civil value, OTP code, menu radio value, or navigation item. */
  value?: string | null;
  /** A number field's parsed value, or `null` while its text does not parse. */
  numberValue?: number | null;
  /** Free-form editing text a picker retains. */
  inputValue?: string;
  /** Whether a surface is open. */
  open?: boolean;
  /** Which menubar menu is open, or `null` for a closed bar. */
  openIndex?: number | null;
  /** The menubar menu that now owns the single Tab stop. */
  focusedIndex?: number;
  /** The calendar day that owns the single Tab stop, or the focused navigation item. */
  focused?: string | null;
  /** The month a calendar is displaying, as `YYYY-MM`. */
  month?: string;
  /** The number field's exact editing text. */
  text?: string;
  /** Whether the number field's editing text parses into range. */
  valid?: boolean;
  /** Whether a number-field scrub gesture is in flight. */
  scrubbing?: boolean;
  readOnly?: boolean;
  required?: boolean;
  /** The range of rows a collection is virtualizing. */
  visibleRange?: VisibleRange;
  /** Selected table rows as inclusive `[start, end]` ranges. */
  selectedRanges?: number[][];
  /** The table's sort state, or `null` once it is cleared. */
  sort?: TableSortState | null;
  /** The table's active cell, or `null` once it is cleared. */
  activeCell?: TableCell | null;
  /** Retained widths of the resizable columns, keyed by declared identifier. */
  columnWidths?: ColumnWidth[];
  /** Declared column identifiers in their current display order. */
  columnOrder?: string[];
  /** The inline edit the core just ended. */
  editEnded?: TableEditEndDetails;
  /** Expanded tree node identifiers. */
  expanded?: string[];
  /** The pending tree branch asking for its children. */
  loadChildren?: string;
  /** Toast identifiers the core's queue dismissed. */
  dismissed?: string[];
  /** The queue a toast viewport is showing, newest first. */
  toasts?: ToastStackEntry[];
  /** The avatar load status the core retained: `idle`, `loading`, `loaded`, or `error`. */
  loadingStatus?: string;
  /** The OTP code that just became complete. */
  complete?: string;
  /** A scroll area's clamped offset. */
  offset?: ScrollOffset;
  scrolling?: boolean;
  hovering?: boolean;
  hasOverflowX?: boolean;
  hasOverflowY?: boolean;
  overflowXStart?: boolean;
  overflowXEnd?: boolean;
  overflowYStart?: boolean;
  overflowYEnd?: boolean;
  /** The side a menu popup was really placed on. */
  side?: string;
  /** The cross-axis alignment a menu popup really used. */
  align?: string;
  /** Whether a menu's anchor left the collision viewport entirely. */
  anchorHidden?: boolean;
  /** Whether the roving highlight is on this menu row. */
  highlighted?: boolean;
  /** Whether a menu row refuses activation. */
  disabled?: boolean;
  /** A checkable menu row's state, or `null` for a row that is not checkable. */
  checked?: boolean | null;
  /** The stable identifier of the menu row the core just activated. */
  activated?: string;
  /** The destination a `Menu.LinkItem` carried into the core's own open-URL path. */
  href?: string;
  /** Every value a multiple select holds, in source order. */
  selectedValues?: string[];
  /** Every value a multiple combobox holds as a chip, in chip order. */
  chipValues?: string[];
  /** The label of each chip a multiple combobox holds. */
  chipLabels?: string[];
  /** The joined label text a select's `Value` part renders, or `null` for the placeholder. */
  valueText?: string | null;
  /** The core's own select or combobox part state. */
  state?: PickerPartState;
}

export interface CommitDetails {
  value?: string;
  numberValue?: number;
  inputValue?: string;
  row?: number;
  column?: number;
}

export interface AnchorPlacementDetails {
  side: string;
  align: string;
  anchorHidden: boolean;
  anchorWidth: number;
  anchorHeight: number;
  availableWidth: number;
  availableHeight: number;
}

/** The composed text of an `input` event, or the empty string. */
export function inputValue(event: QuickGuiEvent): string {
  return event.value ?? "";
}

export function keyEventFromEvent(event: QuickGuiEvent): KeyEventDetails | undefined {
  const value = event.value;
  return value === undefined ? undefined : (JSON.parse(value) as KeyEventDetails);
}

export function mouseEventFromEvent(event: QuickGuiEvent): MouseEventDetails | undefined {
  const value = event.value;
  return value === undefined ? undefined : (JSON.parse(value) as MouseEventDetails);
}

export function wheelEventFromEvent(event: QuickGuiEvent): WheelEventDetails | undefined {
  const value = event.value;
  return value === undefined ? undefined : (JSON.parse(value) as WheelEventDetails);
}

export function gestureEventFromEvent(event: QuickGuiEvent): GestureEventDetails | undefined {
  const value = event.value;
  return value === undefined ? undefined : (JSON.parse(value) as GestureEventDetails);
}

export function dropEventFromEvent(event: QuickGuiEvent): DropEventDetails | undefined {
  const value = event.value;
  return value === undefined ? undefined : (JSON.parse(value) as DropEventDetails);
}

export function capturedPointerFromEvent(event: QuickGuiEvent): CapturedPointerEvent | undefined {
  const value = event.value;
  return value === undefined ? undefined : (JSON.parse(value) as CapturedPointerEvent);
}

export function terminalStatusFromEvent(event: QuickGuiEvent): TerminalStatusEvent | undefined {
  const value = event.value;
  return value === undefined ? undefined : (JSON.parse(value) as TerminalStatusEvent);
}

export function menuSelectionFromEvent(event: QuickGuiEvent): MenuSelectDetails | undefined {
  const value = event.value;
  return value === undefined ? undefined : (JSON.parse(value) as MenuSelectDetails);
}

export function componentChangeFromEvent(event: QuickGuiEvent): ComponentChangeDetails | undefined {
  const value = event.value;
  return value === undefined ? undefined : (JSON.parse(value) as ComponentChangeDetails);
}

export function commitFromEvent(event: QuickGuiEvent): CommitDetails | undefined {
  const value = event.value;
  return value === undefined ? undefined : (JSON.parse(value) as CommitDetails);
}

/** The binding id an `action` event carries, or `undefined` when the payload is missing. */
export function actionFromEvent(event: QuickGuiEvent): string | undefined {
  const value = event.value;
  return value === undefined || value.length === 0 ? undefined : value;
}
