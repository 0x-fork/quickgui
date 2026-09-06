/**
 * Labelling and validation composition for one form control, and semantic groups of fields.
 * The Rust core owns the accessible relationships and the validation timing.
 */

import { NativeNodeTag, NativePart, type NativeNode, type QuickGuiEvent } from "@quickgui/native";
import { EVENT_COMPONENT_CHANGE } from "@quickgui/native/native-tree";

import {
  CODE_DIRTY,
  CODE_DISABLED,
  CODE_FILLED,
  CODE_INVALID,
  CODE_OPEN,
  CODE_REQUIRED,
  CODE_TOUCHED,
  CODE_VALIDATION_DEBOUNCE_TIME,
  CODE_VALIDATION_MESSAGE,
  CODE_VALIDATION_MODE,
} from "./generated.ts";
import { componentChangeFromEvent, type FieldValidationDetails } from "./index.ts";
import {
  applyInputPart,
  createComponentScope,
  createPart,
  createPartContext,
  createViewPart,
  finishPart,
  requireContext,
  resolveBoolean,
  setPart,
  type InputPartProps,
  type PartProps,
} from "./parts.ts";
import { createRenderEffect, useContext, type Accessor } from "./reactive.ts";
import { setExplicitBool, setListener, setMilliseconds, setString } from "./runtime.ts";

export type { FieldValidationDelays, FieldValidationTriggers } from "./index.ts";

export type FieldValidation = FieldValidationDetails;

export interface FieldRootProps extends PartProps {
  invalid?: Accessor<boolean>;
  required?: boolean;
  touched?: Accessor<boolean>;
  dirty?: Accessor<boolean>;
  filled?: Accessor<boolean>;
  /** Bounded message retained for form reports and native accessibility. */
  validationMessage?: Accessor<string>;
  /** Which triggers validate. `"onSubmit"` by default, as in Base UI. */
  validationMode?: "onSubmit" | "onBlur" | "onChange";
  /** How long the core waits after a change before validating, in milliseconds. */
  validationDebounceTime?: number;
  /** The core's own answers for the declared validation mode. */
  onValidationChange?: (validation: FieldValidation, event: QuickGuiEvent) => void;
}

export interface FieldValidityProps extends PartProps {
  /** Whether the readout is shown. Defaults to `true`. */
  visible?: boolean;
}

export interface FieldLabelProps extends PartProps {
  /** Name the control without forwarding pointer activation to it. */
  passive?: boolean;
}

export type FieldControlElement = "input" | "textarea" | "button" | "view" | "text";

export interface FieldControlProps extends InputPartProps {
  /** Native element this control renders. Defaults to `input`. */
  element?: FieldControlElement;
}

class FieldsetState {
  readonly props: PartProps;

  constructor(props: PartProps) {
    this.props = props;
  }

  disabled(): boolean {
    return resolveBoolean(this.props.disabled) === true;
  }
}

const FieldsetContext = createPartContext<FieldsetState>();

class FieldState {
  readonly scope: string;
  readonly props: FieldRootProps;
  readonly fieldset: FieldsetState | undefined;

  constructor(props: FieldRootProps, fieldset: FieldsetState | undefined) {
    this.scope = createComponentScope("qg-field");
    this.props = props;
    this.fieldset = fieldset;
  }

  disabled(): boolean {
    if (resolveBoolean(this.props.disabled) === true) return true;
    const fieldset = this.fieldset;
    return fieldset !== undefined && fieldset.disabled();
  }

  /** The declaration every field part repeats so the core resolves the field without a registry. */
  applyTo(node: NativeNode, part: string): void {
    setPart(node, part, this.scope, undefined);
    const props = this.props;
    createRenderEffect(() => {
      setExplicitBool(node, CODE_DISABLED, this.disabled());
      setExplicitBool(node, CODE_INVALID, readFlag(props.invalid));
      setExplicitBool(node, CODE_REQUIRED, props.required === true);
      setExplicitBool(node, CODE_TOUCHED, readFlag(props.touched));
      setExplicitBool(node, CODE_DIRTY, readFlag(props.dirty));
      setExplicitBool(node, CODE_FILLED, readFlag(props.filled));
      const message = props.validationMessage;
      setString(node, CODE_VALIDATION_MESSAGE, message === undefined ? undefined : message());
    });
  }
}

function readFlag(value: Accessor<boolean> | undefined): boolean {
  return value !== undefined && value();
}

const FieldContext = createPartContext<FieldState>();

function controlTag(element: FieldControlElement | undefined): number {
  if (element === "button") return NativeNodeTag.Button;
  if (element === "view" || element === "text") return NativeNodeTag.View;
  return NativeNodeTag.Input;
}

/** Compound parts for one labelled, validated control. */
export class Field {
  /** Controlled, unstyled labelling and validation composition for one form control. */
  static Root(props: FieldRootProps): NativeNode {
    const state = new FieldState(props, useContext(FieldsetContext));
    const node = createViewPart(props);
    state.applyTo(node, NativePart.Field);
    if (props.validationMode !== undefined) setString(node, CODE_VALIDATION_MODE, props.validationMode);
    if (props.validationDebounceTime !== undefined) {
      setMilliseconds(node, CODE_VALIDATION_DEBOUNCE_TIME, props.validationDebounceTime);
    }
    const onValidationChange = props.onValidationChange;
    if (onValidationChange !== undefined) {
      setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
        const details = componentChangeFromEvent(event);
        if (details === undefined) return;
        const triggers = details.validation;
        const delay = details.validationDelay;
        if (triggers !== undefined && delay !== undefined) {
          onValidationChange({ triggers, delay }, event);
        }
      });
    }
    return FieldContext.provide(state, () => finishPart(node, props));
  }

  /** Visible label. The core forwards its clicks to the control unless `passive` is declared. */
  static Label(props: FieldLabelProps): NativeNode {
    const state = requireContext(FieldContext, "Field.Label", "Field.Root");
    const node = createViewPart(props);
    state.applyTo(node, props.passive === true ? NativePart.FieldPassiveLabel : NativePart.FieldLabel);
    return finishPart(node, props);
  }

  /**
   * The labelled control itself. The core part sets this element's identity, so the control must
   * be the part rather than a wrapper around one; `element` selects which native element it is.
   */
  static Control(props: FieldControlProps): NativeNode {
    const state = requireContext(FieldContext, "Field.Control", "Field.Root");
    const node = createPart(controlTag(props.element), props);
    if (props.element === "textarea") applyInputPart(node, { ...props, multiline: true });
    else if (props.element === undefined || props.element === "input") applyInputPart(node, props);
    state.applyTo(node, NativePart.FieldControl);
    return finishPart(node, props);
  }

  /** Supplementary help described to assistive technology by the core. */
  static Description(props: PartProps): NativeNode {
    const state = requireContext(FieldContext, "Field.Description", "Field.Root");
    const node = createViewPart(props);
    state.applyTo(node, NativePart.FieldDescription);
    return finishPart(node, props);
  }

  /** One field item, Base UI's structural row inside a field. */
  static Item(props: PartProps): NativeNode {
    const state = requireContext(FieldContext, "Field.Item", "Field.Root");
    const node = createViewPart(props);
    state.applyTo(node, NativePart.FieldItem);
    return finishPart(node, props);
  }

  /** Live validity readout; the core owns which triggers answer and how long it waits. */
  static Validity(props: FieldValidityProps): NativeNode {
    const state = requireContext(FieldContext, "Field.Validity", "Field.Root");
    const node = createViewPart(props);
    state.applyTo(node, NativePart.FieldValidity);
    setExplicitBool(node, CODE_OPEN, props.visible ?? true);
    return finishPart(node, props);
  }

  /** Visible error. The core removes it from layout while the controlled field is valid. */
  static Error(props: PartProps): NativeNode {
    const state = requireContext(FieldContext, "Field.Error", "Field.Root");
    const node = createViewPart(props);
    state.applyTo(node, NativePart.FieldError);
    return finishPart(node, props);
  }
}

/** Compound parts for a semantic field group. */
export class Fieldset {
  /** Controlled group semantics for related fields. */
  static Root(props: PartProps): NativeNode {
    const state = new FieldsetState(props);
    const node = createViewPart(props);
    setPart(node, NativePart.Fieldset, createComponentScope("qg-fieldset"), undefined);
    createRenderEffect(() => {
      setExplicitBool(node, CODE_DISABLED, state.disabled());
    });
    return FieldsetContext.provide(state, () => finishPart(node, props));
  }

  /** Group legend named to assistive technology by the core. */
  static Legend(props: PartProps): NativeNode {
    const node = createViewPart(props);
    setPart(node, NativePart.FieldsetLegend, undefined, undefined);
    return finishPart(node, props);
  }

  /** Group description named to assistive technology by the core. */
  static Description(props: PartProps): NativeNode {
    const node = createViewPart(props);
    setPart(node, NativePart.FieldsetDescription, undefined, undefined);
    return finishPart(node, props);
  }

  /** Direct group control that inherits the fieldset's disabled state. */
  static Control(props: FieldControlProps): NativeNode {
    const fieldset = useContext(FieldsetContext);
    const node = createPart(controlTag(props.element), props);
    if (props.element === "textarea") applyInputPart(node, { ...props, multiline: true });
    else if (props.element === undefined || props.element === "input") applyInputPart(node, props);
    setPart(node, NativePart.FieldsetControl, undefined, undefined);
    createRenderEffect(() => {
      const disabled = resolveBoolean(props.disabled) === true || (fieldset !== undefined && fieldset.disabled());
      setExplicitBool(node, CODE_DISABLED, disabled);
    });
    return finishPart(node, props);
  }
}
