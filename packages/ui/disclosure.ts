/**
 * Disclosure components: a single collapsible and an accordion of items. The Rust core owns the
 * expanded state's semantics and whether a closed panel is mounted at all.
 */

import { NativePart, type NativeNode, type QuickGuiEvent } from "@quickgui/native";
import { EVENT_CLICK } from "@quickgui/native/native-tree";

import { CODE_DISABLED, CODE_HEADING_LEVEL, CODE_ITEM_INDEX, CODE_KEEP_MOUNTED, CODE_OPEN } from "./generated.ts";
import {
  createButtonPart,
  createComponentScope,
  createPartContext,
  createViewPart,
  finishPart,
  forwardClick,
  requireContext,
  resolveBoolean,
  setPart,
  type PartProps,
} from "./parts.ts";
import { createRenderEffect, signal, type Accessor } from "./reactive.ts";
import { setExplicitBool, setListener, setNumber } from "./runtime.ts";

// --- collapsible ---------------------------------------------------------------------------------

export interface CollapsibleRootProps extends PartProps {
  open?: Accessor<boolean>;
  defaultOpen?: boolean;
  onOpenChange?: (open: boolean, event: QuickGuiEvent) => void;
  keepMounted?: boolean;
}

class CollapsibleState {
  readonly scope: string;
  readonly props: CollapsibleRootProps;
  readonly _open = signal<boolean>(false);

  constructor(props: CollapsibleRootProps) {
    this.scope = createComponentScope("qg-collapsible");
    this.props = props;
    this._open.write(props.defaultOpen ?? false);
  }

  open(): boolean {
    const controlled = this.props.open;
    return controlled !== undefined ? controlled() : this._open.read();
  }

  toggle(event: QuickGuiEvent): void {
    const next = !this.open();
    if (this.props.open === undefined) this._open.write(next);
    const onOpenChange = this.props.onOpenChange;
    if (onOpenChange !== undefined) onOpenChange(next, event);
  }

  applyTo(node: NativeNode, part: string): void {
    setPart(node, part, this.scope, undefined);
    createRenderEffect(() => {
      setExplicitBool(node, CODE_OPEN, this.open());
    });
    createRenderEffect(() => {
      setExplicitBool(node, CODE_DISABLED, resolveBoolean(this.props.disabled) === true);
    });
    setExplicitBool(node, CODE_KEEP_MOUNTED, this.props.keepMounted === true);
  }
}

const CollapsibleContext = createPartContext<CollapsibleState>();

/** Compound parts for a controlled disclosure. */
export class Collapsible {
  static Root(props: CollapsibleRootProps): NativeNode {
    const state = new CollapsibleState(props);
    const node = createViewPart(props);
    state.applyTo(node, NativePart.Collapsible);
    return CollapsibleContext.provide(state, () => finishPart(node, props));
  }

  /** Disclosure button. Expanded state and the panel relationship come from the core. */
  static Trigger(props: PartProps): NativeNode {
    const state = requireContext(CollapsibleContext, "Collapsible.Trigger", "Collapsible.Root");
    const node = createButtonPart(props);
    state.applyTo(node, NativePart.CollapsibleTrigger);
    setListener(
      node,
      EVENT_CLICK,
      forwardClick(props.onClick, (event: QuickGuiEvent): void => {
        state.toggle(event);
      }),
    );
    return finishPart(node, props);
  }

  /** Disclosure panel. The core omits it, or retains it hidden with `keepMounted`, when closed. */
  static Panel(props: PartProps): NativeNode {
    const state = requireContext(CollapsibleContext, "Collapsible.Panel", "Collapsible.Root");
    const node = createViewPart(props);
    state.applyTo(node, NativePart.CollapsiblePanel);
    return finishPart(node, props);
  }
}

// --- accordion -----------------------------------------------------------------------------------

export interface AccordionRootProps extends PartProps {
  /** Controlled open item values; one entry at most unless `multiple` is declared. */
  value?: Accessor<string[]>;
  defaultValue?: string[];
  onValueChange?: (value: string[], event: QuickGuiEvent) => void;
  multiple?: boolean;
  keepMounted?: boolean;
  /** Heading level for each item header, clamped by the core to 1 through 6. */
  headingLevel?: number;
}

export interface AccordionItemProps extends PartProps {
  value: string;
  /** Caller-visible position projected across this item's parts. */
  index?: number;
}

class AccordionState {
  readonly scope: string;
  readonly props: AccordionRootProps;
  readonly _open = signal<string[]>([]);

  constructor(props: AccordionRootProps) {
    this.scope = createComponentScope("qg-accordion");
    this.props = props;
    this._open.write(props.defaultValue ?? []);
  }

  openValues(): string[] {
    const controlled = this.props.value;
    return controlled !== undefined ? controlled() : this._open.read();
  }

  isOpen(value: string): boolean {
    return this.openValues().includes(value);
  }

  disabled(): boolean {
    return resolveBoolean(this.props.disabled) === true;
  }

  toggle(value: string, event: QuickGuiEvent): void {
    const current = this.openValues();
    let next: string[];
    if (current.includes(value)) next = current.filter((candidate) => candidate !== value);
    else if (this.props.multiple === true) next = [...current, value];
    else next = [value];
    if (this.props.value === undefined) this._open.write(next);
    const onValueChange = this.props.onValueChange;
    if (onValueChange !== undefined) onValueChange(next, event);
  }
}

class AccordionItemState {
  readonly accordion: AccordionState;
  readonly props: AccordionItemProps;

  constructor(accordion: AccordionState, props: AccordionItemProps) {
    this.accordion = accordion;
    this.props = props;
  }

  open(): boolean {
    return this.accordion.isOpen(this.props.value);
  }

  disabled(): boolean {
    return resolveBoolean(this.props.disabled) === true || this.accordion.disabled();
  }

  applyTo(node: NativeNode, part: string): void {
    setPart(node, part, this.accordion.scope, this.props.value);
    setNumber(node, CODE_ITEM_INDEX, this.props.index ?? 0);
    createRenderEffect(() => {
      setExplicitBool(node, CODE_OPEN, this.open());
    });
    createRenderEffect(() => {
      setExplicitBool(node, CODE_DISABLED, this.disabled());
    });
    setExplicitBool(node, CODE_KEEP_MOUNTED, this.accordion.props.keepMounted === true);
    setNumber(node, CODE_HEADING_LEVEL, this.accordion.props.headingLevel ?? 3);
  }
}

const AccordionContext = createPartContext<AccordionState>();
const AccordionItemContext = createPartContext<AccordionItemState>();

/** Compound parts for a controlled accordion. */
export class Accordion {
  /** Controlled, unstyled accordion root supporting single or multiple open items. */
  static Root(props: AccordionRootProps): NativeNode {
    const state = new AccordionState(props);
    const node = createViewPart(props);
    setPart(node, NativePart.Accordion, state.scope, undefined);
    return AccordionContext.provide(state, () => finishPart(node, props));
  }

  /** One accordion item. Its trigger, header, and panel identities derive from this value. */
  static Item(props: AccordionItemProps): NativeNode {
    const accordion = requireContext(AccordionContext, "Accordion.Item", "Accordion.Root");
    const item = new AccordionItemState(accordion, props);
    const node = createViewPart(props);
    item.applyTo(node, NativePart.AccordionItem);
    return AccordionItemContext.provide(item, () => finishPart(node, props));
  }

  /** Accordion heading that contains only this item's trigger. */
  static Header(props: PartProps): NativeNode {
    const item = requireContext(AccordionItemContext, "Accordion.Header", "Accordion.Item");
    const node = createViewPart(props);
    item.applyTo(node, NativePart.AccordionHeader);
    return finishPart(node, props);
  }

  /** Accordion disclosure button for one item. */
  static Trigger(props: PartProps): NativeNode {
    const item = requireContext(AccordionItemContext, "Accordion.Trigger", "Accordion.Item");
    const node = createButtonPart(props);
    item.applyTo(node, NativePart.AccordionTrigger);
    setListener(
      node,
      EVENT_CLICK,
      forwardClick(props.onClick, (event: QuickGuiEvent): void => {
        item.accordion.toggle(item.props.value, event);
      }),
    );
    return finishPart(node, props);
  }

  /** Accordion panel mounted as a named region by the core while its item is open. */
  static Panel(props: PartProps): NativeNode {
    const item = requireContext(AccordionItemContext, "Accordion.Panel", "Accordion.Item");
    const node = createViewPart(props);
    item.applyTo(node, NativePart.AccordionPanel);
    return finishPart(node, props);
  }
}
