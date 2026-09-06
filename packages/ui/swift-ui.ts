/** SwiftUI controls backed by the Rust host and asynchronous native events. */
import { NativeNodeTag, PropertyCode, Window, type NativeNode, type QuickGuiEvent, type ColorValue } from "@quickgui/native";
import { EVENT_CLICK, EVENT_INPUT, EVENT_SUBMIT, EVENT_PRESENTATION_CHANGE } from "@quickgui/native/native-tree";
import { createContext, createRenderEffect, getOwner, onCleanup, runWithOwner, useContext } from "./reactive.ts";
import { bindString, bindNumber, bindExplicitBool, bindPartStyle, type Dynamic, type PartProps } from "./parts.ts";
import { createRenderer, dynamicText, element, insert, setExplicitBool, setInputType, setJson, setListener, setNumber, setString } from "./runtime.ts";
import type { ViewModifier } from "./swift-ui/modifiers.ts";

export type MatchContents =
  | boolean
  | {
      horizontal?: boolean;
      vertical?: boolean;
    };

export interface HostProps extends PartProps {
  /** Size the QuickGUI host from SwiftUI's intrinsic content size on either or both axes. */
  matchContents?: MatchContents;
}

export type ButtonRole = "default" | "cancel" | "destructive";

export interface CommonViewModifierProps {
  /** Stable identifier exposed to macOS accessibility and UI testing. */
  testID?: Dynamic<string>;
  /** SwiftUI modifiers, applied in array order. */
  modifiers?: Dynamic<ViewModifier[]>;
}

export interface ButtonProps extends CommonViewModifierProps {
  /** The text label for a simple button. */
  label?: Dynamic<string>;
  /** An SF Symbols name shown beside `label`. */
  systemImage?: string;
  /** Semantic SwiftUI button role. */
  role?: ButtonRole;
  /** Called when the native SwiftUI button is pressed. */
  onPress?: (event: QuickGuiEvent) => void;
  /** Identifier reserved for widget and live-activity button targets. */
  target?: string;
  /** Optional nested text used as the label when `label` is omitted. */
  children?: () => NativeNode;
  ref?: (node: NativeNode) => void;
}

export interface SliderProps extends CommonViewModifierProps {
  /** Controlled slider value. */
  value: Dynamic<number>;
  min?: Dynamic<number>;
  max?: Dynamic<number>;
  /** Omit for a continuous slider. */
  step?: Dynamic<number>;
  label?: Dynamic<string>;
  onValueChange?: (value: number, event: QuickGuiEvent) => void;
  ref?: (node: NativeNode) => void;
}

export interface ToggleProps extends CommonViewModifierProps {
  /** Controlled SwiftUI `isOn` value. */
  isOn: Dynamic<boolean>;
  label?: Dynamic<string>;
  onIsOnChange?: (isOn: boolean, event: QuickGuiEvent) => void;
  ref?: (node: NativeNode) => void;
}

export interface ProgressViewProps extends CommonViewModifierProps {
  /** Omit for an indeterminate progress indicator. */
  value?: Dynamic<number>;
  total?: Dynamic<number>;
  label?: Dynamic<string>;
  currentValueLabel?: Dynamic<string>;
  ref?: (node: NativeNode) => void;
}

export interface StepperProps extends CommonViewModifierProps {
  /** Controlled stepper value. */
  value: Dynamic<number>;
  min?: Dynamic<number>;
  max?: Dynamic<number>;
  step?: Dynamic<number>;
  label?: Dynamic<string>;
  onValueChange?: (value: number, event: QuickGuiEvent) => void;
  ref?: (node: NativeNode) => void;
}

export interface TextFieldProps extends CommonViewModifierProps {
  /** Controlled field value. */
  value: Dynamic<string>;
  placeholder?: Dynamic<string>;
  onValueChange?: (value: string, event: QuickGuiEvent) => void;
  onSubmit?: (event: QuickGuiEvent) => void;
  ref?: (node: NativeNode) => void;
}

export type PickerStyle =
  | "automatic"
  | "menu"
  | "segmented"
  | "radioGroup"
  | "inline";

export interface PickerOption {
  value: string;
  label: string;
  systemImage?: string;
  disabled?: boolean;
}

export interface PickerProps extends CommonViewModifierProps {
  /** Controlled selected option value. */
  selection: Dynamic<string>;
  options: Dynamic<PickerOption[]>;
  label?: Dynamic<string>;
  style?: PickerStyle;
  onSelectionChange?: (selection: string, event: QuickGuiEvent) => void;
  ref?: (node: NativeNode) => void;
}

export type SegmentedControlRole = "valueSelection" | "tabs";

export interface SegmentedControlProps extends Omit<PickerProps, "style"> {
  /** Use `tabs` for Xcode-style neutral segmented tab navigation on macOS 27. */
  role?: SegmentedControlRole;
}

export type DatePickerComponents = "date" | "hourAndMinute" | "dateAndTime";
export type DatePickerStyle = "automatic" | "field" | "graphical" | "stepperField";

export interface DatePickerProps extends CommonViewModifierProps {
  /** Controlled Unix timestamp in milliseconds. */
  value: Dynamic<number>;
  min?: Dynamic<number>;
  max?: Dynamic<number>;
  label?: Dynamic<string>;
  displayedComponents?: DatePickerComponents;
  style?: DatePickerStyle;
  onValueChange?: (value: number, event: QuickGuiEvent) => void;
  ref?: (node: NativeNode) => void;
}

export interface ColorPickerProps extends CommonViewModifierProps {
  /** Controlled SwiftUI color string, including named colors and CSS-style hex colors. */
  selection: Dynamic<string>;
  label?: Dynamic<string>;
  supportsOpacity?: Dynamic<boolean>;
  onSelectionChange?: (selection: string, event: QuickGuiEvent) => void;
  ref?: (node: NativeNode) => void;
}

export type GaugeStyle =
  | "automatic"
  | "accessoryCircular"
  | "accessoryCircularCapacity"
  | "accessoryLinear"
  | "accessoryLinearCapacity";

export interface GaugeProps extends CommonViewModifierProps {
  value: Dynamic<number>;
  min?: Dynamic<number>;
  max?: Dynamic<number>;
  label?: Dynamic<string>;
  currentValueLabel?: Dynamic<string>;
  minimumValueLabel?: Dynamic<string>;
  maximumValueLabel?: Dynamic<string>;
  style?: GaugeStyle;
  ref?: (node: NativeNode) => void;
}

export interface QuickGUIHostViewProps extends CommonViewModifierProps {
  /** The one ordinary QuickGUI subtree rendered by this reverse host. */
  children?: () => NativeNode;
  /** Fixed SwiftUI frame width and initial QuickGUI viewport width. */
  width?: Dynamic<number>;
  /** Fixed SwiftUI frame height and initial QuickGUI viewport height. */
  height?: Dynamic<number>;
  /** Measure the ordinary QuickGUI subtree on either or both intrinsic axes. */
  matchContents?: MatchContents;
  /** Clear color used by the embedded WGPU surface. Defaults to transparent. */
  background?: ColorValue;
  ref?: (node: NativeNode) => void;
}

export type PopoverAttachmentAnchor =
  "center" | "top" | "bottom" | "leading" | "trailing";

export type PopoverArrowEdge = "top" | "bottom" | "leading" | "trailing";

export interface PopoverProps extends CommonViewModifierProps {
  children?: () => NativeNode;
  isPresented: Dynamic<boolean>;
  onIsPresentedChange?: (isPresented: boolean) => void;
  attachmentAnchor?: PopoverAttachmentAnchor;
  arrowEdge?: PopoverArrowEdge;
  ref?: (node: NativeNode) => void;
}

export interface PopoverTriggerProps {
  /**
   * Native SwiftUI component used as the trigger.
   *
   * This follows Base UI's composition shape: Trigger supplies behavior and `render` replaces its
   * default element. The rendered component's own press handler runs before Trigger's behavior.
   */
  render: () => NativeNode;
  /** Called after the rendered component's own press handler and before the popover opens. */
  onPress?: (event: QuickGuiEvent) => void;
  /** Receives the rendered trigger component rather than the internal structural slot. */
  ref?: (node: NativeNode) => void;
}

export interface PopoverContentProps {
  children?: () => NativeNode;
  ref?: (node: NativeNode) => void;
}

function numeric(node: NativeNode, code: number, value: Dynamic<number> | undefined): void {
  if (value !== undefined) bindNumber(node, code, value);
}

function string(node: NativeNode, code: number, value: Dynamic<string> | undefined): void {
  if (value !== undefined) bindString(node, code, value);
}

function boolean(node: NativeNode, code: number, value: Dynamic<boolean> | undefined): void {
  if (value !== undefined) bindExplicitBool(node, code, value);
}

function make(tag: number, props: CommonViewModifierProps): NativeNode {
  const node = element(tag);
  string(node, PropertyCode.SwiftUITestId, props.testID);
  const modifiers = props.modifiers;
  if (modifiers !== undefined) {
    createRenderEffect(() => {
      const value = typeof modifiers === "function" ? modifiers() : modifiers;
      setJson(node, PropertyCode.SwiftUIModifiers, 16 * 1024, value);
    });
  }
  return node;
}

function finish(node: NativeNode, ref: ((node: NativeNode) => void) | undefined): NativeNode {
  if (ref !== undefined) ref(node);
  return node;
}

function label(node: NativeNode, value: Dynamic<string> | undefined): void {
  if (value !== undefined) insert(node, dynamicText(() => typeof value === "function" ? value() : value));
}

function axes(node: NativeNode, value: MatchContents | undefined): void {
  const horizontal = typeof value === "object" ? value.horizontal === true : value === true;
  const vertical = typeof value === "object" ? value.vertical === true : value === true;
  setExplicitBool(node, PropertyCode.SwiftUIMatchContentsHorizontal, horizontal);
  setExplicitBool(node, PropertyCode.SwiftUIMatchContentsVertical, vertical);
}

/** A native NSHostingView laid out as one QuickGUI leaf. */
export function Host(props: HostProps): NativeNode {
  const node = element(NativeNodeTag.SwiftUIHost);
  const style = props.style;
  if (style !== undefined) bindPartStyle(node, style);
  axes(node, props.matchContents);
  const children = props.children;
  if (children !== undefined) insert(node, children());
  return finish(node, props.ref);
}

export function Button(props: ButtonProps): NativeNode {
  const node = make(NativeNodeTag.SwiftUIButton, props);
  string(node, PropertyCode.Value, props.label);
  setString(node, PropertyCode.SwiftUISystemImage, props.systemImage);
  setString(node, PropertyCode.Role, props.role);
  setString(node, PropertyCode.SwiftUITarget, props.target);
  const onPress = props.onPress;
  if (onPress !== undefined) setListener(node, EVENT_CLICK, onPress);
  const children = props.children;
  if (children !== undefined) insert(node, children());
  return finish(node, props.ref);
}

function numberInput(node: NativeNode, callback: ((value: number, event: QuickGuiEvent) => void) | undefined): void {
  if (callback !== undefined) setListener(node, EVENT_INPUT, (event: QuickGuiEvent): void => {
    const payload = event.value;
    if (payload === undefined) return;
    const value = Number(payload);
    if (Number.isFinite(value)) callback(value, event);
  });
}

function stringInput(node: NativeNode, callback: ((value: string, event: QuickGuiEvent) => void) | undefined): void {
  if (callback !== undefined) setListener(node, EVENT_INPUT, (event: QuickGuiEvent): void => {
    callback(event.value ?? "", event);
  });
}

export function Slider(props: SliderProps): NativeNode {
  const node = make(NativeNodeTag.SwiftUISlider, props);
  numeric(node, PropertyCode.Value, props.value);
  numeric(node, PropertyCode.Minimum, props.min);
  numeric(node, PropertyCode.Maximum, props.max);
  numeric(node, PropertyCode.Step, props.step);
  label(node, props.label);
  numberInput(node, props.onValueChange);
  return finish(node, props.ref);
}

export function Stepper(props: StepperProps): NativeNode {
  const node = make(NativeNodeTag.SwiftUIStepper, props);
  numeric(node, PropertyCode.Value, props.value);
  numeric(node, PropertyCode.Minimum, props.min);
  numeric(node, PropertyCode.Maximum, props.max);
  numeric(node, PropertyCode.Step, props.step);
  label(node, props.label);
  numberInput(node, props.onValueChange);
  return finish(node, props.ref);
}

export function Toggle(props: ToggleProps): NativeNode {
  const node = make(NativeNodeTag.SwiftUIToggle, props);
  boolean(node, PropertyCode.Checked, props.isOn);
  label(node, props.label);
  const onChange = props.onIsOnChange;
  if (onChange !== undefined) setListener(node, EVENT_INPUT, (event: QuickGuiEvent): void => {
    if (event.value === "true" || event.value === "false") onChange(event.value === "true", event);
  });
  return finish(node, props.ref);
}

export function ProgressView(props: ProgressViewProps): NativeNode {
  const node = make(NativeNodeTag.SwiftUIProgressView, props);
  numeric(node, PropertyCode.Value, props.value);
  numeric(node, PropertyCode.Maximum, props.total);
  string(node, PropertyCode.ValueText, props.currentValueLabel);
  label(node, props.label);
  return finish(node, props.ref);
}

function textField(props: TextFieldProps, secure: boolean): NativeNode {
  const node = make(NativeNodeTag.SwiftUITextField, props);
  string(node, PropertyCode.Value, props.value);
  string(node, PropertyCode.Placeholder, props.placeholder);
  setInputType(node, PropertyCode.Password, secure ? "password" : "text");
  stringInput(node, props.onValueChange);
  const onSubmit = props.onSubmit;
  if (onSubmit !== undefined) setListener(node, EVENT_SUBMIT, onSubmit);
  return finish(node, props.ref);
}

export function TextField(props: TextFieldProps): NativeNode { return textField(props, false); }
export function SecureField(props: TextFieldProps): NativeNode { return textField(props, true); }

function picker(props: PickerProps, style: PickerStyle | undefined, role: string | undefined): NativeNode {
  const node = make(NativeNodeTag.SwiftUIPicker, props);
  string(node, PropertyCode.Value, props.selection);
  setString(node, PropertyCode.SwiftUIPickerStyle, style);
  setString(node, PropertyCode.Role, role);
  const options = props.options;
  createRenderEffect(() => {
    setJson(node, PropertyCode.Items, 64 * 1024, typeof options === "function" ? options() : options);
  });
  label(node, props.label);
  stringInput(node, props.onSelectionChange);
  return finish(node, props.ref);
}

export function Picker(props: PickerProps): NativeNode { return picker(props, props.style, undefined); }
export function SegmentedControl(props: SegmentedControlProps): NativeNode { return picker(props, "segmented", props.role); }

function date(node: NativeNode, code: number, value: Dynamic<number> | undefined): void {
  if (value === undefined) return;
  createRenderEffect(() => {
    const current = typeof value === "function" ? value() : value;
    const milliseconds = current;
    if (!Number.isFinite(milliseconds)) throw new TypeError("SwiftUI DatePicker requires a valid Date");
    setString(node, code, String(milliseconds / 1000));
  });
}

export function DatePicker(props: DatePickerProps): NativeNode {
  const node = make(NativeNodeTag.SwiftUIDatePicker, props);
  date(node, PropertyCode.CivilValue, props.value);
  date(node, PropertyCode.CivilMinimum, props.min);
  date(node, PropertyCode.CivilMaximum, props.max);
  setString(node, PropertyCode.SwiftUIDatePickerComponents, props.displayedComponents);
  setString(node, PropertyCode.SwiftUIDatePickerStyle, props.style);
  label(node, props.label);
  const onChange = props.onValueChange;
  if (onChange !== undefined) numberInput(node, (value: number, event: QuickGuiEvent): void => {
    onChange(value * 1000, event);
  });
  return finish(node, props.ref);
}

export function ColorPicker(props: ColorPickerProps): NativeNode {
  const node = make(NativeNodeTag.SwiftUIColorPicker, props);
  string(node, PropertyCode.Value, props.selection);
  boolean(node, PropertyCode.SwiftUIColorSupportsOpacity, props.supportsOpacity);
  label(node, props.label);
  stringInput(node, props.onSelectionChange);
  return finish(node, props.ref);
}

export function Gauge(props: GaugeProps): NativeNode {
  const node = make(NativeNodeTag.SwiftUIGauge, props);
  numeric(node, PropertyCode.Value, props.value);
  numeric(node, PropertyCode.Minimum, props.min);
  numeric(node, PropertyCode.Maximum, props.max);
  string(node, PropertyCode.ValueText, props.currentValueLabel);
  string(node, PropertyCode.SwiftUIGaugeMinimumValueLabel, props.minimumValueLabel);
  string(node, PropertyCode.SwiftUIGaugeMaximumValueLabel, props.maximumValueLabel);
  setString(node, PropertyCode.SwiftUIGaugeStyle, props.style);
  label(node, props.label);
  return finish(node, props.ref);
}

/** An independently owned QuickGUI renderer inside a SwiftUI hierarchy. */
export function QuickGUIHostView(props: QuickGUIHostViewProps): NativeNode {
  const owner = getOwner();
  const parent = Window.getCurrentWindow();
  const node = make(NativeNodeTag.SwiftUIQuickGUIHost, props);
  numeric(node, PropertyCode.Width, props.width);
  numeric(node, PropertyCode.Height, props.height);
  axes(node, props.matchContents);
  let embedded: Window | undefined = undefined;
  let disposed = false;
  // The parent constructor enqueues its native creation after its renderer returns. Queue the
  // reverse host afterwards, including when it is added to an already mounted window.
  queueMicrotask(() => {
    if (disposed || parent.closed) return;
    const match = props.matchContents;
    const horizontal = typeof match === "object" ? match.horizontal === true : match === true;
    const vertical = typeof match === "object" ? match.vertical === true : match === true;
    const width = props.width;
    const height = props.height;
    embedded = Window._createEmbedded(parent, {
      title: "QuickGUI SwiftUI embedded view",
      width: width === undefined ? 320 : typeof width === "function" ? width() : width,
      height: height === undefined ? 200 : typeof height === "function" ? height() : height,
      visible: false,
      decorated: false,
      shadow: false,
      background: props.background ?? "transparent",
      backgroundAppearance: "transparent",
      renderer: (window: Window): (() => void) => runWithOwner(owner, () => createRenderer(() => {
        const surface = element(NativeNodeTag.View);
        const children = props.children;
        if (children !== undefined) insert(surface, children());
        return surface;
      })(window)),
    }, horizontal, vertical);
    setNumber(node, PropertyCode.SwiftUIEmbeddedWindow, embedded.nativeId);
    parent.flush();
    embedded.onClose((_window: Window): void => {
      embedded = undefined;
      if (!disposed) setNumber(node, PropertyCode.SwiftUIEmbeddedWindow, undefined);
    });
  });
  onCleanup(() => {
    disposed = true;
    const window = embedded;
    if (window !== undefined) window.close();
    embedded = undefined;
  });
  return finish(node, props.ref);
}

class PopoverState {
  readonly props: PopoverProps;
  constructor(props: PopoverProps) { this.props = props; }
  presented(): boolean {
    const value = this.props.isPresented;
    return typeof value === "function" ? value() : value;
  }
  request(value: boolean): void {
    const callback = this.props.onIsPresentedChange;
    if (value !== this.presented() && callback !== undefined) callback(value);
  }
}

const PopoverContext = createContext<PopoverState | undefined>(undefined);

function popoverState(): PopoverState {
  const context = useContext(PopoverContext);
  if (context === undefined) throw new Error("SwiftUI Popover parts require a Popover.Root");
  return context;
}

export class Popover {
  static Root(props: PopoverProps): NativeNode {
    const state = new PopoverState(props);
    const node = make(NativeNodeTag.SwiftUIPopover, props);
    boolean(node, PropertyCode.SwiftUIIsPresented, props.isPresented);
    setString(node, PropertyCode.SwiftUIAttachmentAnchor, props.attachmentAnchor ?? "center");
    setString(node, PropertyCode.SwiftUIArrowEdge, props.arrowEdge ?? "bottom");
    setListener(node, EVENT_PRESENTATION_CHANGE, (event: QuickGuiEvent): void => state.request(event.value === "true"));
    return PopoverContext.provide(state, () => {
      const children = props.children;
      if (children !== undefined) insert(node, children());
      return finish(node, props.ref);
    });
  }

  static Trigger(props: PopoverTriggerProps): NativeNode {
    const state = popoverState();
    const node = element(NativeNodeTag.SwiftUIPopoverTrigger);
    const rendered = props.render();
    insert(node, rendered);
    const ref = props.ref;
    if (ref !== undefined) ref(rendered);
    const onPress = props.onPress;
    setListener(node, EVENT_CLICK, (event: QuickGuiEvent): void => {
      if (onPress !== undefined) onPress(event);
      if (!event.defaultPrevented) state.request(true);
    });
    return node;
  }

  static Content(props: PopoverContentProps): NativeNode {
    popoverState();
    const node = element(NativeNodeTag.SwiftUIPopoverContent);
    const children = props.children;
    if (children !== undefined) insert(node, children());
    return finish(node, props.ref);
  }
}

export function PopoverTrigger(props: PopoverTriggerProps): NativeNode { return Popover.Trigger(props); }
export function PopoverContent(props: PopoverContentProps): NativeNode { return Popover.Content(props); }
