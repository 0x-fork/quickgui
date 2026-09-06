/**
 * Pickers: select, constrained combobox, and free-form autocomplete.
 *
 * The option and result lists live in a separate native child window the Rust core paints from
 * the bounded `appearance` declaration, so the popup-side parts are declarations rather than
 * owner-window elements: they name the surface, its placement, its scroll affordances, and the
 * options it holds. Every owner-window part is a real element decorated by the core's own part
 * descriptor. The core owns filtering, highlight movement, typeahead, placement, and lifetime.
 */

import { NativeNodeTag, NativePart, type NativeNode, type QuickGuiEvent } from "@quickgui/native";
import { EVENT_COMMIT, EVENT_COMPONENT_CHANGE, parseColor } from "@quickgui/native/native-tree";

import {
  CODE_ACTIVE_VALUE,
  CODE_ALIGN,
  CODE_ALIGN_ITEM_WITH_TRIGGER,
  CODE_APPEARANCE,
  CODE_AUTO_HIGHLIGHT,
  CODE_FILTER_MODE,
  CODE_HIGHLIGHT_ITEM_ON_HOVER,
  CODE_INPUT_VALUE,
  CODE_ITEM_INDEX,
  CODE_LABEL,
  CODE_LOOP_FOCUS,
  CODE_MODAL,
  CODE_MULTIPLE,
  CODE_OPEN_ON_INPUT_CLICK,
  CODE_OPTIONS,
  CODE_OPTION_GROUP,
  CODE_READ_ONLY,
  CODE_REQUIRED,
  CODE_SIDE,
  CODE_SIDE_OFFSET,
  CODE_VALUES,
  CODE_VALUE_TEXT,
} from "./generated.ts";
import { commitFromEvent, componentChangeFromEvent, type CommitDetails, type PickerPartState } from "./index.ts";
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
import { fragment, setComponentValue, setExplicitBool, setJson, setListener, setNumber, setString } from "./runtime.ts";

// --- declarations --------------------------------------------------------------------------------

/** One entry in a declared select, combobox, or autocomplete option source. */
export interface OptionDeclaration {
  value: string;
  label?: string;
  /** Trailing hint text the core paints on the option row. */
  detail?: string;
  /** Searchable group name. The core's picker has no group rows, so this joins the keywords. */
  group?: string;
  keywords?: string;
  disabled?: boolean;
}

/** Structural geometry and paint for the rows the core renders in its own popover window. */
export interface PickerAppearance {
  width?: number;
  rowHeight?: number;
  maxVisibleRows?: number;
  anchorGap?: number;
  fontSize?: number;
  radius?: number;
  padding?: number;
  verticalPadding?: number;
  background?: number | string;
  color?: number | string;
  highlightBackground?: number | string;
  highlightColor?: number | string;
  selectedBackground?: number | string;
  mutedColor?: number | string;
}

/** The appearance the Rust binding decodes, with every colour packed. */
interface EncodedPickerAppearance {
  width?: number | undefined;
  rowHeight?: number | undefined;
  maxVisibleRows?: number | undefined;
  anchorGap?: number | undefined;
  fontSize?: number | undefined;
  radius?: number | undefined;
  padding?: number | undefined;
  verticalPadding?: number | undefined;
  background?: number | undefined;
  color?: number | undefined;
  highlightBackground?: number | undefined;
  highlightColor?: number | undefined;
  selectedBackground?: number | undefined;
  mutedColor?: number | undefined;
}

function packedColor(value: number | string | undefined): number | undefined {
  return value === undefined ? undefined : parseColor(value);
}

function encodePickerAppearance(appearance: PickerAppearance): EncodedPickerAppearance {
  return {
    width: appearance.width,
    rowHeight: appearance.rowHeight,
    maxVisibleRows: appearance.maxVisibleRows,
    anchorGap: appearance.anchorGap,
    fontSize: appearance.fontSize,
    radius: appearance.radius,
    padding: appearance.padding,
    verticalPadding: appearance.verticalPadding,
    background: packedColor(appearance.background),
    color: packedColor(appearance.color),
    highlightBackground: packedColor(appearance.highlightBackground),
    highlightColor: packedColor(appearance.highlightColor),
    selectedBackground: packedColor(appearance.selectedBackground),
    mutedColor: packedColor(appearance.mutedColor),
  };
}

export type PickerFilterMode = "fuzzy" | "contains" | "startsWith" | "none";

/** The props every picker root shares. */
export interface PickerSourceProps extends PartProps {
  /** Bounded option source. Omit it to declare the options as child `Item` nodes instead. */
  items?: Accessor<OptionDeclaration[]>;
  /** Structural geometry and paint for the rows the core renders in its own window. */
  appearance?: PickerAppearance;
  /**
   * Base UI's `filter`, answered by the core.
   *
   * `"contains"` and `"startsWith"` keep the source order, `"fuzzy"` ranks with the core's own
   * matcher, and `"none"` keeps a source an application or service already filtered.
   */
  filterMode?: PickerFilterMode;
  /** Reported when the core opens or closes its own native popover window. */
  onOpenChange?: (open: boolean, event: QuickGuiEvent) => void;
  /** One edge per commit, even when the committed value did not move. */
  onCommit?: (details: CommitDetails, event: QuickGuiEvent) => void;
}

export interface SelectRootProps extends PickerSourceProps {
  /** Controlled selected option value. */
  value?: Accessor<string | undefined>;
  defaultValue?: string;
  onValueChange?: (value: string | undefined, event: QuickGuiEvent) => void;
  /** Accept more than one value, Base UI's `multiple`. */
  multiple?: boolean;
  /** Controlled value set of a multiple select, bounded by the core's own 256 values. */
  values?: Accessor<string[]>;
  defaultValues?: string[];
  onValuesChange?: (values: string[], event: QuickGuiEvent) => void;
  /** Require a value before form submission. */
  required?: boolean;
  /** Refuse every value change while staying focusable, unlike `disabled`. */
  readOnly?: boolean;
  /** Expect a mounted `Select.Backdrop`; QuickGUI adds no dimming of its own. */
  modal?: boolean;
  /** Line the selected row up with the trigger, Base UI's `alignItemWithTrigger`. */
  alignItemWithTrigger?: boolean;
}

/** The props a picker hosted by a native text input shares. */
export interface PickerInputProps extends PickerSourceProps {
  placeholder?: string;
  /** Seeds the core's retained editing text. The core owns every edit after that. */
  inputValue?: string;
  onInputValueChange?: (value: string, event: QuickGuiEvent) => void;
}

export interface ComboboxRootProps extends PickerInputProps {
  value?: Accessor<string | undefined>;
  defaultValue?: string;
  onValueChange?: (value: string | undefined, event: QuickGuiEvent) => void;
  /** Accept more than one value as chips, Base UI's `multiple`. */
  multiple?: boolean;
  /** Controlled chip set, bounded by the core's own 64 values. */
  values?: Accessor<string[]>;
  defaultValues?: string[];
  onValuesChange?: (values: string[], event: QuickGuiEvent) => void;
  /** Highlight the first result as soon as the query changes. */
  autoHighlight?: boolean;
  /** Open the suggestion surface when the input itself is pressed. Defaults to `true`. */
  openOnInputClick?: boolean;
  /** Move the highlight onto a hovered row. Defaults to `true`. */
  highlightItemOnHover?: boolean;
  /** Wrap the highlight at the ends of the result list. Defaults to `true`. */
  loopFocus?: boolean;
  readOnly?: boolean;
  required?: boolean;
}

export interface AutocompleteRootProps extends PickerInputProps {}

/** A declared picker surface part. The core resolves the real placement in its own window. */
export interface PickerPositionerProps extends PartProps {
  side?: "top" | "bottom" | "left" | "right";
  align?: "start" | "center" | "end";
  sideOffset?: number;
}

/** One declared chip of a multiple combobox. */
export interface ComboboxChipProps extends PartProps {
  /** The chip's position in the set the core retained. */
  index?: number;
}

/** One declared option node. It contributes no element: the core paints every row itself. */
export interface OptionProps extends PartProps {
  /** The option's stable value. */
  value?: string;
  /** Row label. Defaults to the value. */
  label?: string;
  /** Trailing hint text. */
  valueText?: string;
  /** Searchable group name. */
  group?: string;
}

// --- state ---------------------------------------------------------------------------------------

/** Everything the core decided about one select, matching Base UI's trigger `data-*`. */
export interface SelectPartState {
  popupOpen: boolean;
  popupSide: string;
  pressed: boolean;
  placeholder: boolean;
  valid: boolean;
  invalid: boolean;
  dirty: boolean;
  touched: boolean;
  filled: boolean;
  focused: boolean;
  readOnly: boolean;
  required: boolean;
}

function settledSelectState(): SelectPartState {
  return {
    popupOpen: false,
    popupSide: "bottom",
    pressed: false,
    placeholder: true,
    valid: true,
    invalid: false,
    dirty: false,
    touched: false,
    filled: false,
    focused: false,
    readOnly: false,
    required: false,
  };
}

/** Everything the core decided about one combobox, matching Base UI's input `data-*`. */
export interface ComboboxPartState {
  popupOpen: boolean;
  pressed: boolean;
  placeholder: boolean;
  valid: boolean;
  invalid: boolean;
  dirty: boolean;
  touched: boolean;
  filled: boolean;
  focused: boolean;
  readOnly: boolean;
  required: boolean;
  /** The polite live-region text the core derived for `Combobox.Status`. */
  status: string;
  /** Whether the query really matched nothing, which is when `Combobox.Empty` mounts. */
  empty: boolean;
  resultCount: number;
}

function settledComboboxState(): ComboboxPartState {
  return {
    popupOpen: false,
    pressed: false,
    placeholder: true,
    valid: true,
    invalid: false,
    dirty: false,
    touched: false,
    filled: false,
    focused: false,
    readOnly: false,
    required: false,
    status: "",
    empty: false,
    resultCount: 0,
  };
}

/** One chip a multiple combobox holds. */
export interface ComboboxChip {
  value: string;
  label: string;
}

function selectStateFromPart(part: PickerPartState): SelectPartState {
  return {
    popupOpen: part.popupOpen === true,
    popupSide: part.popupSide ?? "bottom",
    pressed: part.pressed === true,
    placeholder: part.placeholder === true,
    valid: part.valid !== false,
    invalid: part.invalid === true,
    dirty: part.dirty === true,
    touched: part.touched === true,
    filled: part.filled === true,
    focused: part.focused === true,
    readOnly: part.readOnly === true,
    required: part.required === true,
  };
}

function comboboxStateFromPart(part: PickerPartState): ComboboxPartState {
  return {
    popupOpen: part.popupOpen === true,
    pressed: part.pressed === true,
    placeholder: part.placeholder === true,
    valid: part.valid !== false,
    invalid: part.invalid === true,
    dirty: part.dirty === true,
    touched: part.touched === true,
    filled: part.filled === true,
    focused: part.focused === true,
    readOnly: part.readOnly === true,
    required: part.required === true,
    status: part.status ?? "",
    empty: part.empty === true,
    resultCount: part.resultCount ?? 0,
  };
}

class PickerState {
  readonly scope: string;
  readonly _select = signal<SelectPartState>(settledSelectState());
  readonly _combobox = signal<ComboboxPartState>(settledComboboxState());
  readonly _valueText = signal<string | null>(null);
  readonly _chips = signal<ComboboxChip[]>([]);

  constructor(prefix: string) {
    this.scope = createComponentScope(prefix);
  }
}

const PickerContext = createPartContext<PickerState>();

function pickerScope(part: string): string {
  const context = useContext(PickerContext);
  if (context === undefined) throw new TypeError(part + " must be used inside its picker root");
  return context.scope;
}

/** Read what the core decided about the enclosing select. */
export function useSelectState(): Accessor<SelectPartState> {
  const context = useContext(PickerContext);
  return context === undefined ? settledSelectState : () => context._select.read();
}

/** Read what the core decided about the enclosing combobox or autocomplete. */
export function useComboboxState(): Accessor<ComboboxPartState> {
  const context = useContext(PickerContext);
  return context === undefined ? settledComboboxState : () => context._combobox.read();
}

/** Read the joined label text a select's `Value` part renders, or `null` for the placeholder. */
export function useSelectValueText(): Accessor<string | null> {
  const context = useContext(PickerContext);
  return context === undefined ? (): string | null => null : () => context._valueText.read();
}

/** Read the chips a multiple combobox holds, in chip order. */
export function useComboboxChips(): Accessor<ComboboxChip[]> {
  const context = useContext(PickerContext);
  return context === undefined ? (): ComboboxChip[] => [] : () => context._chips.read();
}

// --- shared part helpers -------------------------------------------------------------------------

function applyPickerSource(node: NativeNode, props: PickerSourceProps): void {
  const items = props.items;
  if (items !== undefined) {
    createRenderEffect(() => {
      setJson(node, CODE_OPTIONS, 524288, items());
    });
  }
  if (props.appearance !== undefined) setJson(node, CODE_APPEARANCE, 65536, encodePickerAppearance(props.appearance));
  if (props.filterMode !== undefined) setString(node, CODE_FILTER_MODE, props.filterMode);
  const onCommit = props.onCommit;
  if (onCommit !== undefined) {
    setListener(node, EVENT_COMMIT, (event: QuickGuiEvent): void => {
      const details = commitFromEvent(event);
      if (details !== undefined) onCommit(details, event);
    });
  }
}

function createPickerPart(name: string, part: string, props: PartProps, tag: number): NativeNode {
  const scope = pickerScope(name);
  const node = createPart(tag, props);
  setPart(node, part, scope, undefined);
  return finishPart(node, props);
}

function createPositionerPart(name: string, part: string, props: PickerPositionerProps): NativeNode {
  const scope = pickerScope(name);
  const node = createViewPart(props);
  setPart(node, part, scope, undefined);
  setString(node, CODE_SIDE, props.side);
  setString(node, CODE_ALIGN, props.align);
  setNumber(node, CODE_SIDE_OFFSET, props.sideOffset);
  return finishPart(node, props);
}

/** One declared option or group. The core paints the row itself, so this node mounts nothing. */
function createOptionPart(name: string, part: string, props: OptionProps): NativeNode {
  const scope = pickerScope(name);
  const node = createViewPart(props);
  setPart(node, part, scope, props.value);
  if (props.label !== undefined) setString(node, CODE_LABEL, props.label);
  if (props.valueText !== undefined) setString(node, CODE_VALUE_TEXT, props.valueText);
  if (props.group !== undefined) setComponentValue(node, CODE_OPTION_GROUP, props.group);
  return finishPart(node, props);
}

function createChipPart(name: string, part: string, props: ComboboxChipProps, tag: number): NativeNode {
  const scope = pickerScope(name);
  const node = createPart(tag, props);
  setPart(node, part, scope, undefined);
  setNumber(node, CODE_ITEM_INDEX, props.index ?? 0);
  return finishPart(node, props);
}

// --- select --------------------------------------------------------------------------------------

/** Compound parts for a select. */
export class Select {
  /**
   * Controlled select trigger.
   *
   * The trigger is the only element the application declares: the core opens its own native
   * popover window and paints every option row from `appearance`, so no row can ever wait on
   * the application while the core is deciding what a keystroke means.
   */
  static Root(props: SelectRootProps): NativeNode {
    const state = new PickerState("qg-select");
    const uncontrolled = signal<string | undefined>(props.defaultValue);
    const uncontrolledValues = signal<string[]>(props.defaultValues ?? []);
    const value = (): string | undefined => {
      const controlled = props.value;
      return controlled !== undefined ? controlled() : uncontrolled.read();
    };
    const values = (): string[] => {
      const controlled = props.values;
      return controlled !== undefined ? controlled() : uncontrolledValues.read();
    };
    const multiple = props.multiple === true;
    const node = createButtonPart(props);
    setPart(node, NativePart.Select, state.scope, undefined);
    applyPickerSource(node, props);
    createRenderEffect(() => {
      if (multiple) setJson(node, CODE_VALUES, 65536, values());
      else setComponentValue(node, CODE_ACTIVE_VALUE, value());
    });
    setExplicitBool(node, CODE_MULTIPLE, props.multiple);
    setExplicitBool(node, CODE_REQUIRED, props.required);
    setExplicitBool(node, CODE_READ_ONLY, props.readOnly);
    setExplicitBool(node, CODE_MODAL, props.modal);
    setExplicitBool(node, CODE_ALIGN_ITEM_WITH_TRIGGER, props.alignItemWithTrigger);
    setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
      const details = componentChangeFromEvent(event);
      if (details === undefined) return;
      const part = details.state;
      if (part !== undefined) state._select.write(selectStateFromPart(part));
      const valueText = details.valueText;
      if (valueText !== undefined) state._valueText.write(valueText);
      if (multiple) {
        const selected = details.selectedValues;
        if (selected !== undefined) {
          if (props.values === undefined) uncontrolledValues.write(selected);
          const onValuesChange = props.onValuesChange;
          if (onValuesChange !== undefined) onValuesChange(selected, event);
        }
      } else if (details.value !== undefined) {
        const next = details.value === null ? undefined : details.value;
        if (props.value === undefined) uncontrolled.write(next);
        const onValueChange = props.onValueChange;
        if (onValueChange !== undefined) onValueChange(next, event);
      }
      const open = details.open;
      if (open !== undefined) {
        const onOpenChange = props.onOpenChange;
        if (onOpenChange !== undefined) onOpenChange(open, event);
      }
    });
    return PickerContext.provide(state, () => finishPart(node, props));
  }

  /** Base UI's name for the declaration-carrying trigger; `Select.Root` is the same node. */
  static Trigger(props: SelectRootProps): NativeNode {
    return Select.Root(props);
  }

  /** One declared option. The core paints the row itself, so this node mounts nothing. */
  static Option(props: OptionProps): NativeNode {
    return createOptionPart("Select.Option", NativePart.Option, props);
  }

  /** The select's visible label. The trigger points its accessible name at this identity. */
  static Label(props: PartProps): NativeNode {
    return createPickerPart("Select.Label", NativePart.SelectLabel, props, NativeNodeTag.View);
  }

  /** The select's value text. Render `useSelectValueText()`, or the placeholder. */
  static Value(props: PartProps): NativeNode {
    return createPickerPart("Select.Value", NativePart.SelectValue, props, NativeNodeTag.View);
  }

  /** The select's trigger affordance. */
  static Icon(props: PartProps): NativeNode {
    return createPickerPart("Select.Icon", NativePart.SelectIcon, props, NativeNodeTag.View);
  }

  /** An owner-window dimming layer mounted only while the core holds the surface open. */
  static Backdrop(props: PartProps): NativeNode {
    return createPickerPart("Select.Backdrop", NativePart.SelectBackdrop, props, NativeNodeTag.View);
  }

  /** The option-surface boundary. It declares the placement; the core paints the window. */
  static Portal(props: PickerPositionerProps): NativeNode {
    return createPositionerPart("Select.Portal", NativePart.SelectPortal, props);
  }

  /** The option-surface positioner. `side`, `align`, and `sideOffset` are declared here. */
  static Positioner(props: PickerPositionerProps): NativeNode {
    return createPositionerPart("Select.Positioner", NativePart.SelectPositioner, props);
  }

  /** The option surface itself. The core paints it in its own native window. */
  static Popup(props: PartProps): NativeNode {
    return createPickerPart("Select.Popup", NativePart.SelectPopup, props, NativeNodeTag.View);
  }

  /** A decorative arrow on the option surface. */
  static Arrow(props: PartProps): NativeNode {
    return createPickerPart("Select.Arrow", NativePart.SelectArrow, props, NativeNodeTag.View);
  }

  /** The scrolling option list. */
  static List(props: PartProps): NativeNode {
    return createPickerPart("Select.List", NativePart.SelectList, props, NativeNodeTag.View);
  }

  /** One declared option. Its `ItemText` supplies the label when none is declared. */
  static Item(props: OptionProps): NativeNode {
    return createOptionPart("Select.Item", NativePart.SelectItem, props);
  }

  /** One option's visible text. It is the option's label when `label` is omitted. */
  static ItemText(props: PartProps): NativeNode {
    return createPickerPart("Select.ItemText", NativePart.SelectItemText, props, NativeNodeTag.View);
  }

  /** One option's selected mark. */
  static ItemIndicator(props: PartProps): NativeNode {
    return createPickerPart("Select.ItemIndicator", NativePart.SelectItemIndicator, props, NativeNodeTag.View);
  }

  /** An option group. Its label becomes the searchable group name of the options inside it. */
  static Group(props: OptionProps): NativeNode {
    return createOptionPart("Select.Group", NativePart.SelectGroup, props);
  }

  /** An option group's label. */
  static GroupLabel(props: PartProps): NativeNode {
    return createPickerPart("Select.GroupLabel", NativePart.SelectGroupLabel, props, NativeNodeTag.View);
  }

  /** A divider between option groups. */
  static Separator(props: PartProps): NativeNode {
    return createPickerPart("Select.Separator", NativePart.SelectSeparator, props, NativeNodeTag.View);
  }

  /**
   * Declares the upward scroll affordance the core mounts inside its option surface.
   *
   * While the pointer rests on it the option window advances one row every 50 ms, each step an
   * exact one-shot deadline armed by the previous one.
   */
  static ScrollUpArrow(props: PartProps): NativeNode {
    return createPickerPart("Select.ScrollUpArrow", NativePart.SelectScrollUpArrow, props, NativeNodeTag.View);
  }

  /** Declares the downward scroll affordance the core mounts inside its option surface. */
  static ScrollDownArrow(props: PartProps): NativeNode {
    return createPickerPart("Select.ScrollDownArrow", NativePart.SelectScrollDownArrow, props, NativeNodeTag.View);
  }
}

// --- combobox ------------------------------------------------------------------------------------

function comboboxInputProps(props: PickerInputProps): InputPartProps {
  return {
    ref: props.ref,
    style: props.style,
    disabled: props.disabled,
    role: props.role,
    ariaLabel: props.ariaLabel,
    tabIndex: props.tabIndex,
    onClick: props.onClick,
    onMouseEnter: props.onMouseEnter,
    onMouseLeave: props.onMouseLeave,
    onMouseDown: props.onMouseDown,
    onMouseUp: props.onMouseUp,
    onKeyDown: props.onKeyDown,
    onKeyUp: props.onKeyUp,
    onFocus: props.onFocus,
    onBlur: props.onBlur,
    placeholder: props.placeholder,
  };
}

/** Compound parts for a constrained combobox. */
export class Combobox {
  /**
   * Controlled constrained combobox.
   *
   * Arbitrary text is an editing query, not a committable value: the core restores the last
   * committed label on dismissal and reports the constrained value it committed. The input is
   * a leaf, so the compound's other parts are siblings of it rather than children.
   */
  static Root(props: ComboboxRootProps): NativeNode {
    const state = new PickerState("qg-combobox");
    const uncontrolled = signal<string | undefined>(props.defaultValue);
    const uncontrolledValues = signal<string[]>(props.defaultValues ?? []);
    const value = (): string | undefined => {
      const controlled = props.value;
      return controlled !== undefined ? controlled() : uncontrolled.read();
    };
    const values = (): string[] => {
      const controlled = props.values;
      return controlled !== undefined ? controlled() : uncontrolledValues.read();
    };
    const multiple = props.multiple === true;
    const inputProps = comboboxInputProps(props);
    const input = createPart(NativeNodeTag.Input, inputProps);
    applyInputPart(input, inputProps);
    setPart(input, NativePart.Combobox, state.scope, undefined);
    applyPickerSource(input, props);
    createRenderEffect(() => {
      setComponentValue(input, CODE_ACTIVE_VALUE, value());
    });
    if (multiple) {
      createRenderEffect(() => {
        setJson(input, CODE_VALUES, 65536, values());
      });
    }
    if (props.inputValue !== undefined) setString(input, CODE_INPUT_VALUE, props.inputValue);
    setExplicitBool(input, CODE_MULTIPLE, props.multiple);
    setExplicitBool(input, CODE_AUTO_HIGHLIGHT, props.autoHighlight);
    setExplicitBool(input, CODE_OPEN_ON_INPUT_CLICK, props.openOnInputClick);
    setExplicitBool(input, CODE_HIGHLIGHT_ITEM_ON_HOVER, props.highlightItemOnHover);
    setExplicitBool(input, CODE_LOOP_FOCUS, props.loopFocus);
    setExplicitBool(input, CODE_READ_ONLY, props.readOnly);
    setExplicitBool(input, CODE_REQUIRED, props.required);
    setListener(input, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
      const details = componentChangeFromEvent(event);
      if (details === undefined) return;
      const part = details.state;
      if (part !== undefined) state._combobox.write(comboboxStateFromPart(part));
      const chipValues = details.chipValues;
      if (chipValues !== undefined) {
        const labels = details.chipLabels ?? [];
        const chips: ComboboxChip[] = [];
        for (let index = 0; index < chipValues.length; index += 1) {
          const chip = chipValues[index]!;
          chips.push({ value: chip, label: index < labels.length ? labels[index]! : chip });
        }
        state._chips.write(chips);
        if (multiple) {
          if (props.values === undefined) uncontrolledValues.write(chipValues);
          const onValuesChange = props.onValuesChange;
          if (onValuesChange !== undefined) onValuesChange(chipValues, event);
        }
      }
      if (details.value !== undefined) {
        const next = details.value === null ? undefined : details.value;
        if (props.value === undefined) uncontrolled.write(next);
        const onValueChange = props.onValueChange;
        if (onValueChange !== undefined) onValueChange(next, event);
      }
      const inputValue = details.inputValue;
      if (inputValue !== undefined) {
        const onInputValueChange = props.onInputValueChange;
        if (onInputValueChange !== undefined) onInputValueChange(inputValue, event);
      }
      const open = details.open;
      if (open !== undefined) {
        const onOpenChange = props.onOpenChange;
        if (onOpenChange !== undefined) onOpenChange(open, event);
      }
    });
    const ref = props.ref;
    if (ref !== undefined) ref(input);
    return PickerContext.provide(state, () => {
      const children = props.children;
      return fragment(children === undefined ? [input] : [input, children()]);
    });
  }

  /** Base UI's name for the declaration-carrying input; `Combobox.Root` is the same node. */
  static Input(props: ComboboxRootProps): NativeNode {
    return Combobox.Root(props);
  }

  static Option(props: OptionProps): NativeNode {
    return createOptionPart("Combobox.Option", NativePart.Option, props);
  }

  /** The combobox's visible label. */
  static Label(props: PartProps): NativeNode {
    return createPickerPart("Combobox.Label", NativePart.ComboboxLabel, props, NativeNodeTag.View);
  }

  /** The combobox's committed-value text. */
  static Value(props: PartProps): NativeNode {
    return createPickerPart("Combobox.Value", NativePart.ComboboxValue, props, NativeNodeTag.View);
  }

  /** The combobox's affordance glyph. */
  static Icon(props: PartProps): NativeNode {
    return createPickerPart("Combobox.Icon", NativePart.ComboboxIcon, props, NativeNodeTag.View);
  }

  /** The wrapper holding the input, its chips, and its affordances. */
  static InputGroup(props: PartProps): NativeNode {
    return createPickerPart("Combobox.InputGroup", NativePart.ComboboxInputGroup, props, NativeNodeTag.View);
  }

  /** The clear control. The core clears the committed value and every chip. */
  static Clear(props: PartProps): NativeNode {
    return createPickerPart("Combobox.Clear", NativePart.ComboboxClear, props, NativeNodeTag.Button);
  }

  /**
   * The surface trigger.
   *
   * QuickGUI's combobox opens from its own input, so this part carries Base UI's button
   * semantics and the `controls` relationship while the input keeps the opening behavior.
   */
  static Trigger(props: PartProps): NativeNode {
    return createPickerPart("Combobox.Trigger", NativePart.ComboboxTrigger, props, NativeNodeTag.Button);
  }

  /** The chip container of a multiple combobox. */
  static Chips(props: PartProps): NativeNode {
    return createPickerPart("Combobox.Chips", NativePart.ComboboxChips, props, NativeNodeTag.View);
  }

  /** One chip. `index` names the chip position the core retained. */
  static Chip(props: ComboboxChipProps): NativeNode {
    return createChipPart("Combobox.Chip", NativePart.ComboboxChip, props, NativeNodeTag.View);
  }

  /** One chip's remove control. The core removes the chip and reports the new set. */
  static ChipRemove(props: ComboboxChipProps): NativeNode {
    return createChipPart("Combobox.ChipRemove", NativePart.ComboboxChipRemove, props, NativeNodeTag.Button);
  }

  /** An owner-window dimming layer mounted only while the core holds the surface open. */
  static Backdrop(props: PartProps): NativeNode {
    return createPickerPart("Combobox.Backdrop", NativePart.ComboboxBackdrop, props, NativeNodeTag.View);
  }

  /** The suggestion-surface boundary. */
  static Portal(props: PickerPositionerProps): NativeNode {
    return createPositionerPart("Combobox.Portal", NativePart.ComboboxPortal, props);
  }

  /** The suggestion-surface positioner. */
  static Positioner(props: PickerPositionerProps): NativeNode {
    return createPositionerPart("Combobox.Positioner", NativePart.ComboboxPositioner, props);
  }

  /** The suggestion surface itself. The core paints it in its own native window. */
  static Popup(props: PartProps): NativeNode {
    return createPickerPart("Combobox.Popup", NativePart.ComboboxPopup, props, NativeNodeTag.View);
  }

  /** A decorative arrow on the suggestion surface. */
  static Arrow(props: PartProps): NativeNode {
    return createPickerPart("Combobox.Arrow", NativePart.ComboboxArrow, props, NativeNodeTag.View);
  }

  /** The polite live region. Render `useComboboxState()`'s `status` inside it. */
  static Status(props: PartProps): NativeNode {
    return createPickerPart("Combobox.Status", NativePart.ComboboxStatus, props, NativeNodeTag.View);
  }

  /** The no-results part. The core mounts it only while the query really matched nothing. */
  static Empty(props: PartProps): NativeNode {
    return createPickerPart("Combobox.Empty", NativePart.ComboboxEmpty, props, NativeNodeTag.View);
  }

  /** The scrolling result list. */
  static List(props: PartProps): NativeNode {
    return createPickerPart("Combobox.List", NativePart.ComboboxList, props, NativeNodeTag.View);
  }

  /** A grid-shaped result row. */
  static Row(props: PartProps): NativeNode {
    return createPickerPart("Combobox.Row", NativePart.ComboboxRow, props, NativeNodeTag.View);
  }

  /** One declared result. */
  static Item(props: OptionProps): NativeNode {
    return createOptionPart("Combobox.Item", NativePart.ComboboxItem, props);
  }

  /** One result's selected mark. */
  static ItemIndicator(props: PartProps): NativeNode {
    return createPickerPart("Combobox.ItemIndicator", NativePart.ComboboxItemIndicator, props, NativeNodeTag.View);
  }

  /** A result group. Its label becomes the searchable group name of the results inside it. */
  static Group(props: OptionProps): NativeNode {
    return createOptionPart("Combobox.Group", NativePart.ComboboxGroup, props);
  }

  /** A result group's label. */
  static GroupLabel(props: PartProps): NativeNode {
    return createPickerPart("Combobox.GroupLabel", NativePart.ComboboxGroupLabel, props, NativeNodeTag.View);
  }

  /** A wrapper around the mounted rows. */
  static Collection(props: PartProps): NativeNode {
    return createPickerPart("Combobox.Collection", NativePart.ComboboxCollection, props, NativeNodeTag.View);
  }

  /** A divider between result groups. */
  static Separator(props: PartProps): NativeNode {
    return createPickerPart("Combobox.Separator", NativePart.ComboboxSeparator, props, NativeNodeTag.View);
  }
}

// --- autocomplete --------------------------------------------------------------------------------

/**
 * Compound parts for a free-form autocomplete.
 *
 * The core's autocomplete shares the combobox's owner-window parts, so the same `Label`,
 * `Value`, `Icon`, `InputGroup`, `Clear`, `Status`, and `Empty` components mount here. Chips,
 * `multiple`, `readOnly`, and `required` belong to the constrained combobox only.
 */
export class Autocomplete {
  /**
   * Free-form autocomplete.
   *
   * `inputValue` seeds the core's retained text; every edit after that belongs to the core,
   * which reports the exact value it holds through `onInputValueChange`.
   */
  static Root(props: AutocompleteRootProps): NativeNode {
    const state = new PickerState("qg-autocomplete");
    const inputProps = comboboxInputProps(props);
    const input = createPart(NativeNodeTag.Input, inputProps);
    applyInputPart(input, inputProps);
    setPart(input, NativePart.Autocomplete, state.scope, undefined);
    applyPickerSource(input, props);
    if (props.inputValue !== undefined) setString(input, CODE_INPUT_VALUE, props.inputValue);
    setListener(input, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
      const details = componentChangeFromEvent(event);
      if (details === undefined) return;
      const part = details.state;
      if (part !== undefined) state._combobox.write(comboboxStateFromPart(part));
      const inputValue = details.inputValue;
      if (inputValue !== undefined) {
        const onInputValueChange = props.onInputValueChange;
        if (onInputValueChange !== undefined) onInputValueChange(inputValue, event);
      }
      const open = details.open;
      if (open !== undefined) {
        const onOpenChange = props.onOpenChange;
        if (onOpenChange !== undefined) onOpenChange(open, event);
      }
    });
    const ref = props.ref;
    if (ref !== undefined) ref(input);
    return PickerContext.provide(state, () => {
      const children = props.children;
      return fragment(children === undefined ? [input] : [input, children()]);
    });
  }

  /** Base UI's name for the declaration-carrying input; `Autocomplete.Root` is the same node. */
  static Input(props: AutocompleteRootProps): NativeNode {
    return Autocomplete.Root(props);
  }

  static Option(props: OptionProps): NativeNode {
    return createOptionPart("Autocomplete.Option", NativePart.Option, props);
  }

  static Label(props: PartProps): NativeNode {
    return createPickerPart("Autocomplete.Label", NativePart.ComboboxLabel, props, NativeNodeTag.View);
  }

  static Value(props: PartProps): NativeNode {
    return createPickerPart("Autocomplete.Value", NativePart.ComboboxValue, props, NativeNodeTag.View);
  }

  static Icon(props: PartProps): NativeNode {
    return createPickerPart("Autocomplete.Icon", NativePart.ComboboxIcon, props, NativeNodeTag.View);
  }

  static InputGroup(props: PartProps): NativeNode {
    return createPickerPart("Autocomplete.InputGroup", NativePart.ComboboxInputGroup, props, NativeNodeTag.View);
  }

  static Clear(props: PartProps): NativeNode {
    return createPickerPart("Autocomplete.Clear", NativePart.ComboboxClear, props, NativeNodeTag.Button);
  }

  static Portal(props: PickerPositionerProps): NativeNode {
    return createPositionerPart("Autocomplete.Portal", NativePart.ComboboxPortal, props);
  }

  static Positioner(props: PickerPositionerProps): NativeNode {
    return createPositionerPart("Autocomplete.Positioner", NativePart.ComboboxPositioner, props);
  }

  static Popup(props: PartProps): NativeNode {
    return createPickerPart("Autocomplete.Popup", NativePart.ComboboxPopup, props, NativeNodeTag.View);
  }

  static Arrow(props: PartProps): NativeNode {
    return createPickerPart("Autocomplete.Arrow", NativePart.ComboboxArrow, props, NativeNodeTag.View);
  }

  static Status(props: PartProps): NativeNode {
    return createPickerPart("Autocomplete.Status", NativePart.ComboboxStatus, props, NativeNodeTag.View);
  }

  static Empty(props: PartProps): NativeNode {
    return createPickerPart("Autocomplete.Empty", NativePart.ComboboxEmpty, props, NativeNodeTag.View);
  }

  static List(props: PartProps): NativeNode {
    return createPickerPart("Autocomplete.List", NativePart.ComboboxList, props, NativeNodeTag.View);
  }

  static Item(props: OptionProps): NativeNode {
    return createOptionPart("Autocomplete.Item", NativePart.ComboboxItem, props);
  }

  static ItemIndicator(props: PartProps): NativeNode {
    return createPickerPart("Autocomplete.ItemIndicator", NativePart.ComboboxItemIndicator, props, NativeNodeTag.View);
  }

  static Group(props: OptionProps): NativeNode {
    return createOptionPart("Autocomplete.Group", NativePart.ComboboxGroup, props);
  }

  static GroupLabel(props: PartProps): NativeNode {
    return createPickerPart("Autocomplete.GroupLabel", NativePart.ComboboxGroupLabel, props, NativeNodeTag.View);
  }

  static Collection(props: PartProps): NativeNode {
    return createPickerPart("Autocomplete.Collection", NativePart.ComboboxCollection, props, NativeNodeTag.View);
  }

  static Separator(props: PartProps): NativeNode {
    return createPickerPart("Autocomplete.Separator", NativePart.ComboboxSeparator, props, NativeNodeTag.View);
  }
}
