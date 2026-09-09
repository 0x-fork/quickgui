import {
  NativeNode,
  PropertyCode,
  type QuickGuiEvent,
  Window,
  setNativeProperty,
  type ColorValue,
} from "@quickgui/native";
import { createContext, getOwner, onCleanup, runWithOwner, untrack, useContext } from "solid-js";
import { createElement, createRenderer, spread, type JSX } from "./index.ts";
import type { ViewModifier } from "./swift-ui/modifiers.ts";

export type MatchContents =
  | boolean
  | {
      horizontal?: boolean;
      vertical?: boolean;
    };

export interface HostProps extends JSX.NativeProps {
  /** Size the QuickGUI host from SwiftUI's intrinsic content size on either or both axes. */
  matchContents?: MatchContents;
}

export type ButtonRole = "default" | "cancel" | "destructive";

export interface CommonViewModifierProps {
  /** Stable identifier exposed to macOS accessibility and UI testing. */
  testID?: string;
  /** SwiftUI modifiers, applied in array order. */
  modifiers?: ViewModifier[];
}

export interface ButtonProps extends CommonViewModifierProps {
  /** The text label for a simple button. */
  label?: string;
  /** An SF Symbols name shown beside `label`. */
  systemImage?: string;
  /** Semantic SwiftUI button role. */
  role?: ButtonRole;
  /** Called when the native SwiftUI button is pressed. */
  onPress?: (event: QuickGuiEvent) => void;
  /** Identifier reserved for widget and live-activity button targets. */
  target?: string;
  /** Optional nested text used as the label when `label` is omitted. */
  children?: unknown;
  ref?: ((node: NativeNode) => void) | NativeNode;
}

export interface SliderProps extends CommonViewModifierProps {
  /** Controlled slider value. */
  value: number;
  min?: number;
  max?: number;
  /** Omit for a continuous slider. */
  step?: number;
  label?: string;
  onValueChange?: (value: number, event: QuickGuiEvent) => void;
  ref?: ((node: NativeNode) => void) | NativeNode;
}

export interface ToggleProps extends CommonViewModifierProps {
  /** Controlled SwiftUI `isOn` value. */
  isOn: boolean;
  label?: string;
  onIsOnChange?: (isOn: boolean, event: QuickGuiEvent) => void;
  ref?: ((node: NativeNode) => void) | NativeNode;
}

export interface ProgressViewProps extends CommonViewModifierProps {
  /** Omit for an indeterminate progress indicator. */
  value?: number;
  total?: number;
  label?: string;
  currentValueLabel?: string;
  ref?: ((node: NativeNode) => void) | NativeNode;
}

export interface StepperProps extends CommonViewModifierProps {
  /** Controlled stepper value. */
  value: number;
  min?: number;
  max?: number;
  step?: number;
  label?: string;
  onValueChange?: (value: number, event: QuickGuiEvent) => void;
  ref?: ((node: NativeNode) => void) | NativeNode;
}

export interface TextFieldProps extends CommonViewModifierProps {
  /** Controlled field value. */
  value: string;
  placeholder?: string;
  onValueChange?: (value: string, event: QuickGuiEvent) => void;
  onSubmit?: (event: QuickGuiEvent) => void;
  ref?: ((node: NativeNode) => void) | NativeNode;
}

export type PickerStyle = "automatic" | "menu" | "segmented" | "radioGroup" | "inline";

export interface PickerOption {
  value: string;
  label: string;
  systemImage?: string;
  disabled?: boolean;
}

export interface PickerProps extends CommonViewModifierProps {
  /** Controlled selected option value. */
  selection: string;
  options: readonly PickerOption[];
  label?: string;
  style?: PickerStyle;
  onSelectionChange?: (selection: string, event: QuickGuiEvent) => void;
  ref?: ((node: NativeNode) => void) | NativeNode;
}

export type SegmentedControlRole = "valueSelection" | "tabs";

export interface SegmentedControlProps extends Omit<PickerProps, "style"> {
  /** Use `tabs` for Xcode-style neutral segmented tab navigation on macOS 27. */
  role?: SegmentedControlRole;
}

export type DatePickerComponents = "date" | "hourAndMinute" | "dateAndTime";
export type DatePickerStyle = "automatic" | "field" | "graphical" | "stepperField";

export interface DatePickerProps extends CommonViewModifierProps {
  /** Controlled JavaScript date. */
  value: Date;
  min?: Date;
  max?: Date;
  label?: string;
  displayedComponents?: DatePickerComponents;
  style?: DatePickerStyle;
  onValueChange?: (value: Date, event: QuickGuiEvent) => void;
  ref?: ((node: NativeNode) => void) | NativeNode;
}

export interface ColorPickerProps extends CommonViewModifierProps {
  /** Controlled SwiftUI color string, including named colors and CSS-style hex colors. */
  selection: string;
  label?: string;
  supportsOpacity?: boolean;
  onSelectionChange?: (selection: string, event: QuickGuiEvent) => void;
  ref?: ((node: NativeNode) => void) | NativeNode;
}

export type GaugeStyle =
  | "automatic"
  | "accessoryCircular"
  | "accessoryCircularCapacity"
  | "accessoryLinear"
  | "accessoryLinearCapacity";

export interface GaugeProps extends CommonViewModifierProps {
  value: number;
  min?: number;
  max?: number;
  label?: string;
  currentValueLabel?: string;
  minimumValueLabel?: string;
  maximumValueLabel?: string;
  style?: GaugeStyle;
  ref?: ((node: NativeNode) => void) | NativeNode;
}

export interface QuickGUIHostViewProps extends CommonViewModifierProps {
  /** The one ordinary QuickGUI subtree rendered by this reverse host. */
  children?: unknown;
  /** Fixed SwiftUI frame width and initial QuickGUI viewport width. */
  width?: number;
  /** Fixed SwiftUI frame height and initial QuickGUI viewport height. */
  height?: number;
  /** Measure the ordinary QuickGUI subtree on either or both intrinsic axes. */
  matchContents?: MatchContents;
  /** Clear color used by the embedded WGPU surface. Defaults to transparent. */
  background?: ColorValue;
  ref?: ((node: NativeNode) => void) | NativeNode;
}

export type PopoverAttachmentAnchor = "center" | "top" | "bottom" | "leading" | "trailing";

export type PopoverArrowEdge = "top" | "bottom" | "leading" | "trailing";

export interface PopoverProps extends CommonViewModifierProps {
  children?: unknown;
  isPresented: boolean;
  onIsPresentedChange?: (isPresented: boolean) => void;
  attachmentAnchor?: PopoverAttachmentAnchor;
  arrowEdge?: PopoverArrowEdge;
  ref?: ((node: NativeNode) => void) | NativeNode;
}

export interface PopoverTriggerProps {
  /**
   * Native SwiftUI component used as the trigger.
   *
   * This follows Base UI's composition shape: Trigger supplies behavior and `render` replaces its
   * default element. The rendered component's own press handler runs before Trigger's behavior.
   */
  render: JSX.Element;
  /** Called after the rendered component's own press handler and before the popover opens. */
  onPress?: (event: QuickGuiEvent) => void;
  /** Receives the rendered trigger component rather than the internal structural slot. */
  ref?: (node: NativeNode) => void;
}

export interface PopoverContentProps {
  children?: unknown;
  ref?: ((node: NativeNode) => void) | NativeNode;
}

interface PopoverContextValue {
  isPresented: () => boolean;
  requestPresentation: (isPresented: boolean) => void;
}

const PopoverContext = createContext<PopoverContextValue>();

/** An AppKit `NSHostingView` embedded as one QuickGUI native-layout leaf. */
export function Host(props: HostProps): NativeNode {
  const node = createElement("swift-ui-host");
  spread(node, props);
  return node;
}

/** A real SwiftUI `Button`. It must be nested under a SwiftUI {@link Host}. */
export function Button(props: ButtonProps): NativeNode {
  const node = createElement("swift-ui-button");
  spread(node, props);
  return node;
}

/** A real controlled SwiftUI `Slider`. It must be nested under a SwiftUI {@link Host}. */
export function Slider(props: SliderProps): NativeNode {
  const node = createElement("swift-ui-slider");
  const handleInput = (event: QuickGuiEvent) => {
    const value = Number(event.value);
    if (Number.isFinite(value)) props.onValueChange?.(value, event);
  };
  spread(node, {
    get value() {
      return props.value;
    },
    get min() {
      return props.min;
    },
    get max() {
      return props.max;
    },
    get step() {
      return props.step;
    },
    get modifiers() {
      return props.modifiers;
    },
    get testID() {
      return props.testID;
    },
    get onInput() {
      return props.onValueChange ? handleInput : undefined;
    },
    get children() {
      return props.label;
    },
    get ref() {
      return props.ref;
    },
  });
  return node;
}

/** A real controlled SwiftUI `Toggle`. It must be nested under a SwiftUI {@link Host}. */
export function Toggle(props: ToggleProps): NativeNode {
  const node = createElement("swift-ui-toggle");
  const handleInput = (event: QuickGuiEvent) => {
    if (event.value === "true") props.onIsOnChange?.(true, event);
    else if (event.value === "false") props.onIsOnChange?.(false, event);
  };
  spread(node, {
    get checked() {
      return props.isOn;
    },
    get modifiers() {
      return props.modifiers;
    },
    get testID() {
      return props.testID;
    },
    get onInput() {
      return props.onIsOnChange ? handleInput : undefined;
    },
    get children() {
      return props.label;
    },
    get ref() {
      return props.ref;
    },
  });
  return node;
}

/** A determinate or indeterminate native SwiftUI `ProgressView`. */
export function ProgressView(props: ProgressViewProps): NativeNode {
  const node = createElement("swift-ui-progress-view");
  spread(node, {
    get value() {
      return props.value;
    },
    get max() {
      return props.total;
    },
    get valueText() {
      return props.currentValueLabel;
    },
    get modifiers() {
      return props.modifiers;
    },
    get testID() {
      return props.testID;
    },
    get children() {
      return props.label;
    },
    get ref() {
      return props.ref;
    },
  });
  return node;
}

/** A real controlled SwiftUI `Stepper`. It must be nested under a SwiftUI {@link Host}. */
export function Stepper(props: StepperProps): NativeNode {
  const node = createElement("swift-ui-stepper");
  const handleInput = (event: QuickGuiEvent) => {
    const value = Number(event.value);
    if (Number.isFinite(value)) props.onValueChange?.(value, event);
  };
  spread(node, {
    get value() {
      return props.value;
    },
    get min() {
      return props.min;
    },
    get max() {
      return props.max;
    },
    get step() {
      return props.step;
    },
    get modifiers() {
      return props.modifiers;
    },
    get testID() {
      return props.testID;
    },
    get onInput() {
      return props.onValueChange ? handleInput : undefined;
    },
    get children() {
      return props.label;
    },
    get ref() {
      return props.ref;
    },
  });
  return node;
}

function createTextField(props: TextFieldProps, secure: boolean): NativeNode {
  const node = createElement("swift-ui-text-field");
  const handleInput = (event: QuickGuiEvent) => props.onValueChange?.(event.value ?? "", event);
  spread(node, {
    get value() {
      return props.value;
    },
    get placeholder() {
      return props.placeholder;
    },
    type: secure ? "password" : "text",
    get modifiers() {
      return props.modifiers;
    },
    get testID() {
      return props.testID;
    },
    get onInput() {
      return props.onValueChange ? handleInput : undefined;
    },
    get onSubmit() {
      return props.onSubmit;
    },
    get ref() {
      return props.ref;
    },
  });
  return node;
}

/** A real controlled SwiftUI `TextField`. It must be nested under a SwiftUI {@link Host}. */
export function TextField(props: TextFieldProps): NativeNode {
  return createTextField(props, false);
}

/** A real controlled SwiftUI `SecureField`. It must be nested under a SwiftUI {@link Host}. */
export function SecureField(props: TextFieldProps): NativeNode {
  return createTextField(props, true);
}

function createPicker(
  props: PickerProps,
  forcedStyle?: () => PickerStyle | undefined,
  forcedRole?: () => string | undefined,
): NativeNode {
  const node = createElement("swift-ui-picker");
  const handleInput = (event: QuickGuiEvent) => props.onSelectionChange?.(event.value ?? "", event);
  spread(node, {
    get value() {
      return props.selection;
    },
    get items() {
      return props.options;
    },
    get pickerStyle() {
      return forcedStyle?.() ?? props.style;
    },
    get role() {
      return forcedRole?.();
    },
    get modifiers() {
      return props.modifiers;
    },
    get testID() {
      return props.testID;
    },
    get onInput() {
      return props.onSelectionChange ? handleInput : undefined;
    },
    get children() {
      return props.label;
    },
    get ref() {
      return props.ref;
    },
  });
  return node;
}

/** A controlled native SwiftUI `Picker`. */
export function Picker(props: PickerProps): NativeNode {
  return createPicker(props);
}

/** A controlled SwiftUI picker using the native segmented style. */
export function SegmentedControl(props: SegmentedControlProps): NativeNode {
  return createPicker(
    props,
    () => "segmented",
    () => props.role,
  );
}

function epochSeconds(value: Date | undefined, property: string): string | undefined {
  if (value === undefined) return undefined;
  const milliseconds = value.getTime();
  if (!Number.isFinite(milliseconds)) {
    throw new TypeError(`SwiftUI DatePicker ${property} must be a valid Date`);
  }
  return String(milliseconds / 1_000);
}

/** A controlled native SwiftUI `DatePicker`. */
export function DatePicker(props: DatePickerProps): NativeNode {
  const node = createElement("swift-ui-date-picker");
  const handleInput = (event: QuickGuiEvent) => {
    const seconds = Number(event.value);
    if (Number.isFinite(seconds)) {
      props.onValueChange?.(new Date(seconds * 1_000), event);
    }
  };
  spread(node, {
    get civilValue() {
      return epochSeconds(props.value, "value");
    },
    get civilMinimum() {
      return epochSeconds(props.min, "min");
    },
    get civilMaximum() {
      return epochSeconds(props.max, "max");
    },
    get datePickerComponents() {
      return props.displayedComponents;
    },
    get datePickerStyle() {
      return props.style;
    },
    get modifiers() {
      return props.modifiers;
    },
    get testID() {
      return props.testID;
    },
    get onInput() {
      return props.onValueChange ? handleInput : undefined;
    },
    get children() {
      return props.label;
    },
    get ref() {
      return props.ref;
    },
  });
  return node;
}

/** A controlled native SwiftUI `ColorPicker`. */
export function ColorPicker(props: ColorPickerProps): NativeNode {
  const node = createElement("swift-ui-color-picker");
  const handleInput = (event: QuickGuiEvent) => props.onSelectionChange?.(event.value ?? "", event);
  spread(node, {
    get value() {
      return props.selection;
    },
    get supportsOpacity() {
      return props.supportsOpacity;
    },
    get modifiers() {
      return props.modifiers;
    },
    get testID() {
      return props.testID;
    },
    get onInput() {
      return props.onSelectionChange ? handleInput : undefined;
    },
    get children() {
      return props.label;
    },
    get ref() {
      return props.ref;
    },
  });
  return node;
}

/** A native SwiftUI `Gauge`. */
export function Gauge(props: GaugeProps): NativeNode {
  const node = createElement("swift-ui-gauge");
  spread(node, {
    get value() {
      return props.value;
    },
    get min() {
      return props.min;
    },
    get max() {
      return props.max;
    },
    get valueText() {
      return props.currentValueLabel;
    },
    get gaugeMinimumValueLabel() {
      return props.minimumValueLabel;
    },
    get gaugeMaximumValueLabel() {
      return props.maximumValueLabel;
    },
    get gaugeStyle() {
      return props.style;
    },
    get modifiers() {
      return props.modifiers;
    },
    get testID() {
      return props.testID;
    },
    get children() {
      return props.label;
    },
    get ref() {
      return props.ref;
    },
  });
  return node;
}

function matchAxes(value: MatchContents | undefined): {
  horizontal: boolean;
  vertical: boolean;
} {
  if (value === true) return { horizontal: true, vertical: true };
  if (!value || typeof value !== "object") {
    return { horizontal: false, vertical: false };
  }
  return {
    horizontal: value.horizontal === true,
    vertical: value.vertical === true,
  };
}

/**
 * Hosts exactly one ordinary QuickGUI subtree inside SwiftUI.
 *
 * The subtree owns a normal retained Rust renderer and native input/accessibility surface. Use a
 * QuickGUI `View` when several siblings are needed, matching Expo's single-child RNHostView rule.
 */
export function QuickGUIHostView(props: QuickGUIHostViewProps): NativeNode {
  const owner = getOwner();
  const parent = Window.getCurrentWindow();
  const axes = matchAxes(props.matchContents);
  const node = createElement("swift-ui-quickgui-host");
  spread(node, {
    get width() {
      return props.width;
    },
    get height() {
      return props.height;
    },
    get matchContents() {
      return props.matchContents;
    },
    get testID() {
      return props.testID;
    },
    get ref() {
      return props.ref;
    },
  });

  let embedded: Window | undefined;
  let disposing = false;
  const cancelNativeReady = parent._afterNativeReady(() => {
    if (disposing) return;
    embedded = Window._createEmbedded(
      parent,
      {
        title: "QuickGUI SwiftUI embedded view",
        width: props.width ?? 320,
        height: props.height ?? 200,
        minimumSize: null,
        visible: false,
        decorated: false,
        shadow: false,
        background: props.background ?? "transparent",
        backgroundAppearance: "transparent",
        renderer: (window) =>
          runWithOwner(owner, () =>
            createRenderer(() => {
              const surface = createElement("view");
              spread(surface, {
                get children() {
                  return props.children;
                },
              });
              return surface;
            })(window),
          ),
      },
      axes,
    );
    setNativeProperty(node, PropertyCode.SwiftUIEmbeddedWindow, embedded.nativeId);
    // Initial reverse hosts mount while their owner Window constructor is still completing. Flush
    // the association before application code enters its hosted event wait.
    parent.flush();
    embedded.onClose(() => {
      embedded = undefined;
      if (!disposing) {
        setNativeProperty(node, PropertyCode.SwiftUIEmbeddedWindow, null);
      }
    });
  });

  onCleanup(() => {
    disposing = true;
    cancelNativeReady();
    embedded?.close();
    embedded = undefined;
  });
  return node;
}

function PopoverRoot(props: PopoverProps): NativeNode {
  const context: PopoverContextValue = {
    isPresented: () => props.isPresented,
    requestPresentation(isPresented) {
      if (isPresented !== props.isPresented) {
        props.onIsPresentedChange?.(isPresented);
      }
    },
  };

  const resolveRoot = PopoverContext({
    value: context,
    get children() {
      const node = createElement("swift-ui-popover");
      spread(node, {
        get isPresented() {
          return props.isPresented;
        },
        get attachmentAnchor() {
          return props.attachmentAnchor ?? "center";
        },
        get arrowEdge() {
          return props.arrowEdge ?? "bottom";
        },
        get testID() {
          return props.testID;
        },
        get modifiers() {
          return props.modifiers;
        },
        onPresentationChange(event: { value?: string }) {
          context.requestPresentation(event.value === "true");
        },
        get children() {
          return props.children;
        },
        get ref() {
          return props.ref;
        },
      });
      return node;
    },
  }) as unknown as () => NativeNode;
  // The provider exposes its child as an accessor. Resolve the stable root node once; every
  // reactive Popover property is tracked by `spread` below.
  return untrack(resolveRoot);
}

function requirePopoverContext(component: string): PopoverContextValue {
  const context = useContext(PopoverContext);
  if (!context) {
    throw new TypeError(`${component} must be used inside <Popover>`);
  }
  return context;
}

export function PopoverTrigger(props: PopoverTriggerProps): NativeNode {
  const context = requirePopoverContext("Popover.Trigger");
  const node = createElement("swift-ui-popover-trigger");
  let previousRender: NativeNode | undefined;
  spread(node, {
    get children() {
      const rendered = props.render;
      if (!(rendered instanceof NativeNode)) {
        throw new TypeError(
          "Popover.Trigger render must be one @quickgui/solid/swift-ui component",
        );
      }
      if (rendered !== previousRender) {
        previousRender = rendered;
        props.ref?.(rendered);
      }
      return rendered;
    },
    onPress(event: QuickGuiEvent) {
      props.onPress?.(event);
      if (!event.defaultPrevented && !context.isPresented()) {
        context.requestPresentation(true);
      }
    },
  });
  return node;
}

export function PopoverContent(props: PopoverContentProps): NativeNode {
  requirePopoverContext("Popover.Content");
  const node = createElement("swift-ui-popover-content");
  spread(node, props);
  return node;
}

/** Native SwiftUI popover with controlled state and Base UI-style trigger composition. */
export const Popover = Object.assign(PopoverRoot, {
  Trigger: PopoverTrigger,
  Content: PopoverContent,
});
