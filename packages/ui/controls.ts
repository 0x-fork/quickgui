/**
 * Controlled toggle controls: checkbox, checkbox group, radio, switch, toggle, and toggle group.
 *
 * Every part is one host element the Rust core recognizes by its `part` name. The core owns the
 * semantics, keyboard behavior, and derived states; the component only declares the controlled
 * value and forwards the application's styling and listeners.
 */

import { NativePart, type NativeNode, type QuickGuiEvent } from "@quickgui/native";
import { EVENT_CLICK, EVENT_COMPONENT_CHANGE } from "@quickgui/native/native-tree";

import {
  CODE_CHECKED,
  CODE_INDETERMINATE,
  CODE_ITEMS,
  CODE_MULTIPLE,
  CODE_PARENT,
  CODE_PRESSED,
  CODE_READ_ONLY,
  CODE_REQUIRED,
  CODE_VALUES,
} from "./generated.ts";
import { componentChangeFromEvent } from "./index.ts";
import {
  createButtonPart,
  createComponentScope,
  createPartContext,
  createViewPart,
  finishPart,
  forwardClick,
  setPart,
  type PartProps,
} from "./parts.ts";
import { createRenderEffect, signal, useContext, type Accessor } from "./reactive.ts";
import { setExplicitBool, setJson, setListener } from "./runtime.ts";
import type { ComponentItem } from "./types.ts";

export type CheckedState = boolean | "indeterminate";

// --- checkbox ------------------------------------------------------------------------------------

export interface CheckboxProps extends PartProps {
  /** Inside a `CheckboxGroup.Root`, the declared value this checkbox toggles. */
  value?: string;
  /** Inside a `CheckboxGroup.Root`, make this the group's derived parent checkbox. */
  parent?: boolean;
  /**
   * A standalone parent checkbox's children, as checked booleans. The core folds them into on,
   * mixed, or off, so the mixed state is derived rather than retained anywhere.
   */
  childrenChecked?: Accessor<boolean[]>;
  /** Refuse changes while keeping the control focusable and its value announced. */
  readOnly?: boolean;
  /** Controlled `true`, `false`, or `"indeterminate"` toggle state. */
  checked?: Accessor<CheckedState>;
  defaultChecked?: CheckedState;
  onCheckedChange?: (checked: boolean, event: QuickGuiEvent) => void;
}

export interface CheckboxGroupProps extends PartProps {
  /** Controlled checked values, in the declared order. */
  value?: Accessor<string[]>;
  defaultValue?: string[];
  onValueChange?: (value: string[], event: QuickGuiEvent) => void;
  /** The complete, ordered universe a parent checkbox derives its state from. */
  allValues?: string[];
}

class CheckboxGroupState {
  readonly scope: string;

  constructor(scope: string) {
    this.scope = scope;
  }
}

const CheckboxGroupContext = createPartContext<CheckboxGroupState>();

/** Compound parts for a controlled checkbox. */
export class Checkbox {
  /**
   * Controlled, unstyled checkbox root carrying the core's exact on/off/mixed toggle state.
   *
   * Inside a `CheckboxGroup.Root` the checkbox becomes a member of that group: `value` names the
   * declared value it toggles, `parent` makes it the group's derived parent checkbox, and the
   * core owns the checked state, the mixed parent state, and the click behavior for both.
   */
  static Root(props: CheckboxProps): NativeNode {
    const group = useContext(CheckboxGroupContext);
    const node = createButtonPart(props);
    if (props.readOnly !== undefined) setExplicitBool(node, CODE_READ_ONLY, props.readOnly);
    if (group !== undefined && (props.value !== undefined || props.parent === true)) {
      const parent = props.parent === true;
      setPart(node, parent ? NativePart.CheckboxGroupParent : NativePart.CheckboxGroupItem, group.scope, parent ? undefined : props.value);
      return finishPart(node, props);
    }
    const uncontrolled = signal<CheckedState>(props.defaultChecked ?? false);
    const checked = (): CheckedState => {
      const controlled = props.checked;
      return controlled !== undefined ? controlled() : uncontrolled.read();
    };
    setPart(node, NativePart.Checkbox, undefined, undefined);
    if (props.parent === true) {
      // A standalone parent checkbox declares its children's checked booleans and the core folds
      // them into on, mixed, or off; nothing derives the mixed state here.
      setExplicitBool(node, CODE_PARENT, true);
      const childrenChecked = props.childrenChecked;
      if (childrenChecked !== undefined) {
        createRenderEffect(() => {
          setJson(node, CODE_VALUES, 65536, childrenChecked());
        });
      }
    }
    createRenderEffect(() => {
      const state = checked();
      setExplicitBool(node, CODE_CHECKED, state === true);
      setExplicitBool(node, CODE_INDETERMINATE, state === "indeterminate");
    });
    setListener(
      node,
      EVENT_CLICK,
      forwardClick(props.onClick, (event: QuickGuiEvent): void => {
        const next = checked() !== true;
        if (props.checked === undefined) uncontrolled.write(next);
        const onCheckedChange = props.onCheckedChange;
        if (onCheckedChange !== undefined) onCheckedChange(next, event);
      }),
    );
    return finishPart(node, props);
  }

  /** Application-owned checkbox mark, hidden from the control's accessible name by the core. */
  static Indicator(props: PartProps): NativeNode {
    const group = useContext(CheckboxGroupContext);
    const node = createViewPart(props);
    if (group !== undefined) setPart(node, NativePart.CheckboxGroupIndicator, group.scope, undefined);
    else setPart(node, NativePart.CheckboxIndicator, undefined, undefined);
    return finishPart(node, props);
  }
}

/** A controlled checkbox group; the core keeps checked values in the declared order. */
export class CheckboxGroup {
  static Root(props: CheckboxGroupProps): NativeNode {
    const scope = createComponentScope("qg-checkbox-group");
    const uncontrolled = signal<string[]>(props.defaultValue ?? []);
    const values = (): string[] => {
      const controlled = props.value;
      return controlled !== undefined ? controlled() : uncontrolled.read();
    };
    const node = createViewPart(props);
    setPart(node, NativePart.CheckboxGroup, scope, undefined);
    createRenderEffect(() => {
      setJson(node, CODE_VALUES, 65536, values());
    });
    if (props.allValues !== undefined) setJson(node, CODE_ITEMS, 65536, props.allValues);
    setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
      const details = componentChangeFromEvent(event);
      if (details === undefined) return;
      const next = details.checkedValues;
      if (next === undefined) return;
      if (props.value === undefined) uncontrolled.write(next);
      const onValueChange = props.onValueChange;
      if (onValueChange !== undefined) onValueChange(next, event);
    });
    return CheckboxGroupContext.provide(new CheckboxGroupState(scope), () => finishPart(node, props));
  }
}

// --- radio ---------------------------------------------------------------------------------------

export interface RadioGroupProps extends PartProps {
  value?: Accessor<string | undefined>;
  defaultValue?: string;
  onValueChange?: (value: string, event: QuickGuiEvent) => void;
  /** Refuse changes while keeping the group focusable. */
  readOnly?: boolean;
  required?: boolean;
}

export interface RadioProps extends PartProps {
  /** Refuse changes while keeping the control focusable. */
  readOnly?: boolean;
  /** Value this radio selects in its `RadioGroup`. */
  value?: string;
  /** Controlled selection for a radio used without a `RadioGroup`. */
  checked?: Accessor<boolean>;
  defaultChecked?: boolean;
  onCheckedChange?: (checked: boolean, event: QuickGuiEvent) => void;
}

class RadioGroupState {
  readonly props: RadioGroupProps;
  readonly _value = signal<string | undefined>(undefined);

  constructor(props: RadioGroupProps) {
    this.props = props;
    this._value.write(props.defaultValue);
  }

  value(): string | undefined {
    const controlled = this.props.value;
    return controlled !== undefined ? controlled() : this._value.read();
  }

  select(next: string, event: QuickGuiEvent): void {
    if (this.props.value === undefined) this._value.write(next);
    const onValueChange = this.props.onValueChange;
    if (onValueChange !== undefined) onValueChange(next, event);
  }
}

const RadioGroupContext = createPartContext<RadioGroupState>();

/** Semantic group for related radio roots. The core supplies roving Tab and arrow behavior. */
export class RadioGroup {
  static Root(props: RadioGroupProps): NativeNode {
    const state = new RadioGroupState(props);
    const node = createViewPart(props);
    setPart(node, NativePart.RadioGroup, undefined, undefined);
    if (props.readOnly !== undefined) setExplicitBool(node, CODE_READ_ONLY, props.readOnly);
    if (props.required !== undefined) setExplicitBool(node, CODE_REQUIRED, props.required);
    return RadioGroupContext.provide(state, () => finishPart(node, props));
  }
}

/** Compound parts for a controlled radio button. */
export class Radio {
  /** Controlled radio root. Inside a `RadioGroup` its selection comes from the group value. */
  static Root(props: RadioProps): NativeNode {
    const group = useContext(RadioGroupContext);
    const uncontrolled = signal<boolean>(props.defaultChecked ?? false);
    const checked = (): boolean => {
      if (group !== undefined) return group.value() === props.value;
      const controlled = props.checked;
      return controlled !== undefined ? controlled() : uncontrolled.read();
    };
    const node = createButtonPart(props);
    setPart(node, NativePart.Radio, undefined, props.value);
    if (props.readOnly !== undefined) setExplicitBool(node, CODE_READ_ONLY, props.readOnly);
    createRenderEffect(() => {
      setExplicitBool(node, CODE_CHECKED, checked());
    });
    setListener(
      node,
      EVENT_CLICK,
      forwardClick(props.onClick, (event: QuickGuiEvent): void => {
        if (group !== undefined) {
          if (props.value !== undefined) group.select(props.value, event);
          return;
        }
        if (props.checked === undefined) uncontrolled.write(true);
        const onCheckedChange = props.onCheckedChange;
        if (onCheckedChange !== undefined) onCheckedChange(true, event);
      }),
    );
    return finishPart(node, props);
  }

  /** Application-owned radio dot, hidden from the control's accessible name by the core. */
  static Indicator(props: PartProps): NativeNode {
    const node = createViewPart(props);
    setPart(node, NativePart.RadioIndicator, undefined, undefined);
    return finishPart(node, props);
  }
}

// --- switch --------------------------------------------------------------------------------------

export interface SwitchProps extends PartProps {
  checked?: Accessor<boolean>;
  defaultChecked?: boolean;
  onCheckedChange?: (checked: boolean, event: QuickGuiEvent) => void;
  /** Refuse changes while keeping the control focusable. */
  readOnly?: boolean;
}

/** Compound parts for a controlled switch. */
export class Switch {
  /** Controlled, unstyled switch root/track carrying the core's switch role. */
  static Root(props: SwitchProps): NativeNode {
    const uncontrolled = signal<boolean>(props.defaultChecked ?? false);
    const checked = (): boolean => {
      const controlled = props.checked;
      return controlled !== undefined ? controlled() : uncontrolled.read();
    };
    const node = createButtonPart(props);
    setPart(node, NativePart.Switch, undefined, undefined);
    if (props.readOnly !== undefined) setExplicitBool(node, CODE_READ_ONLY, props.readOnly);
    createRenderEffect(() => {
      setExplicitBool(node, CODE_CHECKED, checked());
    });
    setListener(
      node,
      EVENT_CLICK,
      forwardClick(props.onClick, (event: QuickGuiEvent): void => {
        const next = !checked();
        if (props.checked === undefined) uncontrolled.write(next);
        const onCheckedChange = props.onCheckedChange;
        if (onCheckedChange !== undefined) onCheckedChange(next, event);
      }),
    );
    return finishPart(node, props);
  }

  /** Application-owned switch thumb, hidden from the control's accessible name by the core. */
  static Thumb(props: PartProps): NativeNode {
    const node = createViewPart(props);
    setPart(node, NativePart.SwitchThumb, undefined, undefined);
    return finishPart(node, props);
  }
}

// --- toggle --------------------------------------------------------------------------------------

export interface ToggleProps extends PartProps {
  pressed?: Accessor<boolean>;
  defaultPressed?: boolean;
  onPressedChange?: (pressed: boolean, event: QuickGuiEvent) => void;
}

/** Compound parts for a toggle button: a button that stays pressed, not a checkbox. */
export class Toggle {
  static Root(props: ToggleProps): NativeNode {
    const uncontrolled = signal<boolean>(props.defaultPressed ?? false);
    const pressed = (): boolean => {
      const controlled = props.pressed;
      return controlled !== undefined ? controlled() : uncontrolled.read();
    };
    const node = createButtonPart(props);
    setPart(node, NativePart.Toggle, undefined, undefined);
    createRenderEffect(() => {
      setExplicitBool(node, CODE_PRESSED, pressed());
    });
    setListener(
      node,
      EVENT_CLICK,
      forwardClick(props.onClick, (event: QuickGuiEvent): void => {
        const next = !pressed();
        if (props.pressed === undefined) uncontrolled.write(next);
        const onPressedChange = props.onPressedChange;
        if (onPressedChange !== undefined) onPressedChange(next, event);
      }),
    );
    return finishPart(node, props);
  }

  /** Application-owned toggle indicator, hidden from the accessible name by the core. */
  static Indicator(props: PartProps): NativeNode {
    const node = createViewPart(props);
    setPart(node, NativePart.ToggleIndicator, undefined, undefined);
    return finishPart(node, props);
  }
}

export interface ToggleGroupProps extends PartProps {
  /** The ordered navigation model. */
  items: ComponentItem[];
  /** Controlled pressed values. */
  value?: Accessor<string[]>;
  defaultValue?: string[];
  onValueChange?: (value: string[], event: QuickGuiEvent) => void;
  /** Allow several pressed items. */
  multiple?: boolean;
}

export interface ToggleGroupItemProps extends PartProps {
  value: string;
}

/**
 * Toggle-group root with single or multiple selection. `items` declares the ordered navigation
 * model and `value` the pressed values; the core owns the selection policy, the roving Tab stop,
 * and disabled-item skipping.
 */
const ToggleGroupContext = createPartContext<string>();

export class ToggleGroup {
  static Root(props: ToggleGroupProps): NativeNode {
    const scope = createComponentScope("qg-toggle-group");
    const uncontrolled = signal<string[]>(props.defaultValue ?? []);
    const pressed = (): string[] => {
      const controlled = props.value;
      return controlled !== undefined ? controlled() : uncontrolled.read();
    };
    const node = createViewPart(props);
    setPart(node, NativePart.ToggleGroup, scope, undefined);
    setJson(node, CODE_ITEMS, 65536, props.items);
    if (props.multiple !== undefined) setExplicitBool(node, CODE_MULTIPLE, props.multiple);
    createRenderEffect(() => {
      setJson(node, CODE_VALUES, 65536, pressed());
    });
    setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
      const details = componentChangeFromEvent(event);
      if (details === undefined) return;
      const next = details.pressed;
      if (next === undefined) return;
      if (props.value === undefined) uncontrolled.write(next);
      const onValueChange = props.onValueChange;
      if (onValueChange !== undefined) onValueChange(next, event);
    });
    return ToggleGroupContext.provide(scope, () => finishPart(node, props));
  }

  /** Application-owned toggle-group item carrying pressed-button semantics from the core. */
  static Item(props: ToggleGroupItemProps): NativeNode {
    const node = createButtonPart(props);
    setPart(node, NativePart.ToggleGroupItem, useContext(ToggleGroupContext), props.value);
    return finishPart(node, props);
  }
}
