/**
 * Toolbar with a single roving Tab stop, and the semantic separator.
 *
 * `items` declares the ordered navigation model. The Rust core answers arrows, Home, and End on
 * the focused item, skips disabled items, and reports the moved Tab stop through
 * `onActiveChange`.
 */

import { NativeNodeTag, NativePart, type NativeNode, type QuickGuiEvent } from "@quickgui/native";
import { EVENT_COMPONENT_CHANGE } from "@quickgui/native/native-tree";

import { CODE_ACTIVE_VALUE, CODE_ITEMS, CODE_LOOP_FOCUS, CODE_ORIENTATION } from "./generated.ts";
import { componentChangeFromEvent } from "./index.ts";
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
import { setComponentValue, setExplicitBool, setJson, setListener, setString } from "./runtime.ts";
import type { ComponentItem } from "./types.ts";

export interface ToolbarRootProps extends PartProps {
  /** The ordered navigation model: one entry per item, in Tab order. */
  items?: ComponentItem[];
  /** Controlled value of the item that owns the roving Tab stop. */
  active?: Accessor<string | undefined>;
  defaultActive?: string;
  orientation?: "horizontal" | "vertical";
  /** Wrap arrow navigation at the ends of the toolbar. */
  loopFocus?: boolean;
  onActiveChange?: (active: string, event: QuickGuiEvent) => void;
}

/** One declared toolbar item part. */
export interface ToolbarItemProps extends PartProps {
  /** The item's stable value, matching an entry in the toolbar's `items`. */
  value?: string;
}

export interface ToolbarInputProps extends InputPartProps {
  /** The value this input owns in its toolbar's `items`. */
  partValue?: string;
  /** Native element this toolbar input renders. Defaults to a text input. */
  element?: "input" | "button" | "view" | "text";
}

/** The instance scope of the nearest `Toolbar.Root`, so every item reaches the same core toolbar. */
const ToolbarContext = createPartContext<string>();

function createToolbarItem(part: string, props: ToolbarItemProps): NativeNode {
  const node = createButtonPart(props);
  setPart(node, part, useContext(ToolbarContext), props.value);
  return finishPart(node, props);
}

function elementTag(element: "input" | "button" | "view" | "text" | undefined): number {
  if (element === "button") return NativeNodeTag.Button;
  if (element === "view") return NativeNodeTag.View;
  if (element === "text") return NativeNodeTag.Text;
  return NativeNodeTag.Input;
}

/** Compound parts for a toolbar. */
export class Toolbar {
  /** Toolbar root. Exactly one enabled item stays in the Tab sequence. */
  static Root(props: ToolbarRootProps): NativeNode {
    const uncontrolled = signal<string | undefined>(props.defaultActive);
    const active = (): string | undefined => {
      const controlled = props.active;
      return controlled !== undefined ? controlled() : uncontrolled.read();
    };
    const scope = createComponentScope("qg-toolbar");
    const node = createViewPart(props);
    setPart(node, NativePart.Toolbar, scope, undefined);
    if (props.items !== undefined) setJson(node, CODE_ITEMS, 65536, props.items);
    createRenderEffect(() => {
      setComponentValue(node, CODE_ACTIVE_VALUE, active());
    });
    if (props.orientation !== undefined) setString(node, CODE_ORIENTATION, props.orientation);
    if (props.loopFocus !== undefined) setExplicitBool(node, CODE_LOOP_FOCUS, props.loopFocus);
    setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
      const details = componentChangeFromEvent(event);
      if (details === undefined) return;
      const next = details.active;
      if (next === undefined || next === null) return;
      if (props.active === undefined) uncontrolled.write(next);
      const onActiveChange = props.onActiveChange;
      if (onActiveChange !== undefined) onActiveChange(next, event);
    });
    return ToolbarContext.provide(scope, () => finishPart(node, props));
  }

  /** Application-owned toolbar item. */
  static Item(props: ToolbarItemProps): NativeNode {
    return createToolbarItem(NativePart.ToolbarItem, props);
  }

  /** One toolbar command, projected with the core's Button role. */
  static Button(props: ToolbarItemProps): NativeNode {
    return createToolbarItem(NativePart.ToolbarButton, props);
  }

  /** One toolbar link, projected with the core's Link role. */
  static Link(props: ToolbarItemProps): NativeNode {
    return createToolbarItem(NativePart.ToolbarLink, props);
  }

  /** One toolbar input. It keeps the roving contract and its own element role. */
  static Input(props: ToolbarInputProps): NativeNode {
    const node = createPart(elementTag(props.element), props);
    if (props.element === undefined || props.element === "input") applyInputPart(node, props);
    setPart(node, NativePart.ToolbarInput, useContext(ToolbarContext), props.partValue);
    return finishPart(node, props);
  }

  /** A related run of toolbar items, projected with the core's Group role. */
  static Group(props: PartProps): NativeNode {
    const node = createViewPart(props);
    setPart(node, NativePart.ToolbarGroup, useContext(ToolbarContext), undefined);
    return finishPart(node, props);
  }

  /** A toolbar separator, whose orientation the core takes from the toolbar's cross axis. */
  static Separator(props: PartProps): NativeNode {
    const node = createViewPart(props);
    setPart(node, NativePart.ToolbarSeparator, useContext(ToolbarContext), undefined);
    return finishPart(node, props);
  }
}

export interface SeparatorProps extends PartProps {
  /** A horizontal rule divides stacked content; a vertical one divides a row. */
  orientation?: "horizontal" | "vertical";
}

/** Unstyled semantic separator. The caller still declares the rule's extent and colour. */
export class Separator {
  static Root(props: SeparatorProps): NativeNode {
    const node = createViewPart(props);
    setPart(node, NativePart.Separator, undefined, undefined);
    if (props.orientation !== undefined) setString(node, CODE_ORIENTATION, props.orientation);
    return finishPart(node, props);
  }
}
