/**
 * Controlled number field. The Rust core parses, clamps, formats, steps, and scrubs; the parts
 * declare the value and receive the core's decisions through `componentchange` and `commit`.
 */

import { NativeNodeTag, NativePart, type NativeNode, type QuickGuiEvent } from "@quickgui/native";
import { EVENT_COMMIT, EVENT_COMPONENT_CHANGE } from "@quickgui/native/native-tree";

import {
  CODE_ALLOW_WHEEL_SCRUB,
  CODE_LARGE_STEP,
  CODE_MAX,
  CODE_MIN,
  CODE_ORIENTATION,
  CODE_PITCH,
  CODE_PRECISION,
  CODE_READ_ONLY,
  CODE_REQUIRED,
  CODE_SMALL_STEP,
  CODE_SNAP_ON_STEP,
  CODE_STEP,
  CODE_VALUES,
} from "./generated.ts";
import { commitFromEvent, componentChangeFromEvent, type CommitDetails } from "./index.ts";
import {
  applyInputPart,
  createButtonPart,
  createComponentScope,
  createPart,
  createPartContext,
  createViewPart,
  finishPart,
  setPart,
  type InputPartProps,
  type PartProps,
} from "./parts.ts";
import { createRenderEffect, signal, useContext, type Accessor } from "./reactive.ts";
import { setExplicitBool, setJson, setListener, setNumber, setString } from "./runtime.ts";

/** Everything the core decided about one number field. */
export interface NumberFieldState {
  /** Whether a scrub gesture is in flight, matching Base UI's `data-scrubbing`. */
  scrubbing: boolean;
  readOnly: boolean;
  required: boolean;
}

function settledNumberField(): NumberFieldState {
  return { scrubbing: false, readOnly: false, required: false };
}

export interface NumberFieldRootProps extends PartProps {
  /** Controlled numeric value. `undefined` for an empty field. */
  value?: Accessor<number | undefined>;
  defaultValue?: number;
  min?: number;
  max?: number;
  step?: number;
  /** Alt-modified step. Defaults to a tenth of `step`. */
  smallStep?: number;
  /** Shift-modified step. Defaults to ten times `step`. */
  largeStep?: number;
  /** Fractional digits used when the core formats a committed value. */
  precision?: number;
  /** Land a stepped value on the step grid. */
  snapOnStep?: boolean;
  /** Step on the wheel while focused. Defaults to `true`. */
  allowWheelScrub?: boolean;
  /** Refuse every change while staying focusable, unlike `disabled`. */
  readOnly?: boolean;
  /** Required for form submission. */
  required?: boolean;
  /** Which axis a scrub gesture reads. Defaults to `"horizontal"`. */
  scrubDirection?: "horizontal" | "vertical" | "both";
  /** Logical pixels per step during a scrub. Defaults to the core's own two. */
  scrubSensitivity?: number;
  onValueChange?: (value: number | undefined, valid: boolean, event: QuickGuiEvent) => void;
  /** The core's own commit boundary, matching Base UI's `onValueCommitted`. */
  onValueCommitted?: (value: number | undefined, event: QuickGuiEvent) => void;
}

export interface NumberFieldInputProps extends InputPartProps {
  /** Return commits: the core clamps into range and reformats before reporting. */
  onCommit?: (details: CommitDetails, event: QuickGuiEvent) => void;
}

class NumberFieldRootState {
  /** The instance scope every number-field part declares. */
  readonly scope = createComponentScope("qg-number-field");
  readonly _state = signal<NumberFieldState>(settledNumberField());

  state(): NumberFieldState {
    return this._state.read();
  }
}

const NumberFieldContext = createPartContext<NumberFieldRootState>();

/** Read the live number-field state inside a `NumberField.Root` subtree. */
export function useNumberFieldState(): Accessor<NumberFieldState> {
  const context = useContext(NumberFieldContext);
  return context === undefined ? settledNumberField : () => context.state();
}

/** The scope of the nearest `NumberField.Root`. */
function numberFieldScope(): string | undefined {
  const context = useContext(NumberFieldContext);
  return context === undefined ? undefined : context.scope;
}

function createNumberFieldPart(tag: number, part: string, props: PartProps): NativeNode {
  const node = createPart(tag, props);
  setPart(node, part, numberFieldScope(), undefined);
  return finishPart(node, props);
}

/** Compound parts for a number field. */
export class NumberField {
  /**
   * Controlled number-field root.
   *
   * The core parses, clamps, formats, and steps; `value` seeds its retained editing text and
   * the result travels back through `onValueChange`.
   */
  static Root(props: NumberFieldRootProps): NativeNode {
    const state = new NumberFieldRootState();
    const uncontrolled = signal<number | undefined>(props.defaultValue);
    const value = (): number | undefined => {
      const controlled = props.value;
      return controlled !== undefined ? controlled() : uncontrolled.read();
    };
    const node = createViewPart(props);
    setPart(node, NativePart.NumberField, state.scope, undefined);
    createRenderEffect(() => {
      const current = value();
      setJson(node, CODE_VALUES, 65536, current === undefined ? [] : [current]);
    });
    if (props.min !== undefined) setNumber(node, CODE_MIN, props.min);
    if (props.max !== undefined) setNumber(node, CODE_MAX, props.max);
    if (props.step !== undefined) setNumber(node, CODE_STEP, props.step);
    if (props.smallStep !== undefined) setNumber(node, CODE_SMALL_STEP, props.smallStep);
    if (props.largeStep !== undefined) setNumber(node, CODE_LARGE_STEP, props.largeStep);
    if (props.precision !== undefined) setNumber(node, CODE_PRECISION, props.precision);
    setExplicitBool(node, CODE_SNAP_ON_STEP, props.snapOnStep);
    setExplicitBool(node, CODE_ALLOW_WHEEL_SCRUB, props.allowWheelScrub);
    setExplicitBool(node, CODE_READ_ONLY, props.readOnly);
    setExplicitBool(node, CODE_REQUIRED, props.required);
    setString(node, CODE_ORIENTATION, props.scrubDirection);
    if (props.scrubSensitivity !== undefined) setNumber(node, CODE_PITCH, props.scrubSensitivity);
    setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
      const details = componentChangeFromEvent(event);
      if (details === undefined) return;
      if (details.text === undefined) return;
      const parsed = details.numberValue;
      const next = parsed === undefined || parsed === null ? undefined : parsed;
      state._state.write({
        scrubbing: details.scrubbing === true,
        readOnly: details.readOnly === true,
        required: details.required === true,
      });
      if (props.value === undefined) uncontrolled.write(next);
      const onValueChange = props.onValueChange;
      if (onValueChange !== undefined) onValueChange(next, details.valid !== false, event);
      // The core's own commit boundary: the value it clamped and reformatted.
      if (details.committed === true) {
        const onValueCommitted = props.onValueCommitted;
        if (onValueCommitted !== undefined) onValueCommitted(next, event);
      }
    });
    return NumberFieldContext.provide(state, () => finishPart(node, props));
  }

  /** Structural group the input and steppers are laid out inside. */
  static Group(props: PartProps): NativeNode {
    return createNumberFieldPart(NativeNodeTag.View, NativePart.NumberFieldGroup, props);
  }

  /** Controlled number-field input. The core owns its editing text, parsing, and commit. */
  static Input(props: NumberFieldInputProps): NativeNode {
    const node = createPart(NativeNodeTag.Input, props);
    applyInputPart(node, props);
    setPart(node, NativePart.NumberFieldInput, numberFieldScope(), undefined);
    const onCommit = props.onCommit;
    if (onCommit !== undefined) {
      setListener(node, EVENT_COMMIT, (event: QuickGuiEvent): void => {
        const details = commitFromEvent(event);
        if (details !== undefined) onCommit(details, event);
      });
    }
    return finishPart(node, props);
  }

  /** Application-owned increment stepper carrying the core's bounded press-and-hold repeat. */
  static Increment(props: PartProps): NativeNode {
    const node = createButtonPart(props);
    setPart(node, NativePart.NumberFieldIncrement, numberFieldScope(), undefined);
    return finishPart(node, props);
  }

  /** Application-owned decrement stepper carrying the core's bounded press-and-hold repeat. */
  static Decrement(props: PartProps): NativeNode {
    const node = createButtonPart(props);
    setPart(node, NativePart.NumberFieldDecrement, numberFieldScope(), undefined);
    return finishPart(node, props);
  }

  /**
   * Scrub area.
   *
   * The core turns the captured drag into whole steps at the declared sensitivity and keeps
   * the unconverted remainder for the gesture, so a slow drag moves one step at a time.
   */
  static ScrubArea(props: PartProps): NativeNode {
    return createNumberFieldPart(NativeNodeTag.View, NativePart.NumberFieldScrubArea, props);
  }

  /** Caller-drawn scrub cursor. Style it from `useNumberFieldState().scrubbing`. */
  static ScrubAreaCursor(props: PartProps): NativeNode {
    return createNumberFieldPart(NativeNodeTag.View, NativePart.NumberFieldScrubAreaCursor, props);
  }
}
