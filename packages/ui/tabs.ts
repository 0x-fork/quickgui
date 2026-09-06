/**
 * Controlled tab set. The Rust core owns roving focus, arrow keys, activation, and which panels
 * are mounted; the component declares the active value and forwards styling.
 */

import { NativePart, type NativeNode, type PopoverPlacement, type QuickGuiEvent } from "@quickgui/native";
import { EVENT_CLICK, EVENT_COMPONENT_CHANGE } from "@quickgui/native/native-tree";

import {
  CODE_ACTIVATE_ON_FOCUS,
  CODE_ACTIVE_VALUE,
  CODE_ANCHOR_PLACEMENT,
  CODE_ITEM_INDEX,
  CODE_KEEP_MOUNTED,
  CODE_LOOP_FOCUS,
  CODE_ORIENTATION,
  CODE_PART_VALUE,
} from "./generated.ts";
import { componentChangeFromEvent, type TabsIndicatorGeometry } from "./index.ts";
import {
  createButtonPart,
  createComponentScope,
  createPartContext,
  createViewPart,
  finishPart,
  forwardClick,
  requireContext,
  setPart,
  type PartProps,
} from "./parts.ts";
import { createRenderEffect, signal, useContext, type Accessor } from "./reactive.ts";
import { setComponentValue, setExplicitBool, setListener, setNumber, setString } from "./runtime.ts";

/** Everything the core decided about one tab set. */
export interface TabsState {
  /** The side the selection travelled toward: `none`, `left`, `right`, `up`, or `down`. */
  activationDirection: string;
  /** The active tab's laid-out box, published by the core during paint; absent without a placement. */
  indicator: TabsIndicatorGeometry | null;
}

function settledTabs(): TabsState {
  return { activationDirection: "none", indicator: null };
}

export interface TabsRootProps extends PartProps {
  value?: Accessor<string | undefined>;
  defaultValue?: string;
  onValueChange?: (value: string, event: QuickGuiEvent) => void;
  orientation?: "horizontal" | "vertical";
  /** `"manual"` activates on Enter or Space; `"automatic"` activates on arrow focus. */
  activation?: "manual" | "automatic";
  /** Wrap arrow navigation at the ends of the tab list. Defaults to `true`. */
  loop?: boolean;
  /** Retain inactive panels as `display: none` instead of omitting them. */
  keepMounted?: boolean;
  /** Everything the core decided about the tab set, whenever it changes. */
  onTabsStateChange?: (state: TabsState, event: QuickGuiEvent) => void;
}

export interface TabsTabProps extends PartProps {
  value: string;
  /** This tab's position, which is how the core knows which way the selection travelled. */
  index?: number;
}

export interface TabsIndicatorProps extends PartProps {
  /** Tab this indicator belongs to. Defaults to the enclosing tab, then the active tab. */
  value?: string;
  /**
   * Anchor the indicator to the active tab's edge, and publish that tab's box back. `"bottom"`
   * draws the familiar underline; without a placement no geometry is reported.
   */
  placement?: PopoverPlacement;
}

export interface TabsPanelProps extends PartProps {
  value: string;
}

class TabsRootState {
  readonly scope: string;
  readonly props: TabsRootProps;
  readonly _value = signal<string | undefined>(undefined);
  readonly _state = signal<TabsState>(settledTabs());

  constructor(props: TabsRootProps) {
    this.scope = createComponentScope("qg-tabs");
    this.props = props;
    this._value.write(props.defaultValue);
  }

  value(): string | undefined {
    const controlled = this.props.value;
    return controlled !== undefined ? controlled() : this._value.read();
  }

  state(): TabsState {
    return this._state.read();
  }

  select(next: string, event: QuickGuiEvent): void {
    if (this.props.value === undefined) this._value.write(next);
    const onValueChange = this.props.onValueChange;
    if (onValueChange !== undefined) onValueChange(next, event);
  }

  /** The declaration every tab part repeats so the Rust binding decodes it without a registry. */
  applyTo(node: NativeNode, part: string, partValue: string | undefined): void {
    setPart(node, part, this.scope, partValue);
    createRenderEffect(() => {
      setComponentValue(node, CODE_ACTIVE_VALUE, this.value());
    });
    setString(node, CODE_ORIENTATION, this.props.orientation ?? "horizontal");
    setExplicitBool(node, CODE_ACTIVATE_ON_FOCUS, (this.props.activation ?? "manual") === "automatic");
    setExplicitBool(node, CODE_LOOP_FOCUS, this.props.loop ?? true);
    setExplicitBool(node, CODE_KEEP_MOUNTED, this.props.keepMounted === true);
  }
}

const TabsContext = createPartContext<TabsRootState>();
const TabValueContext = createPartContext<string>();

/** Read the live tab-set state inside a `Tabs.Root` subtree. */
export function useTabsState(): Accessor<TabsState> {
  const state = useContext(TabsContext);
  return state === undefined ? settledTabs : () => state.state();
}

/** Compound parts for a controlled tab set. */
export class Tabs {
  /** Controlled, unstyled tab set. The core owns roving focus, arrow keys, and panel mounting. */
  static Root(props: TabsRootProps): NativeNode {
    const state = new TabsRootState(props);
    const node = createViewPart(props);
    state.applyTo(node, NativePart.Tabs, undefined);
    setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
      const details = componentChangeFromEvent(event);
      if (details === undefined) return;
      const direction = details.activationDirection;
      if (direction === undefined || direction === null) return;
      const next: TabsState = { activationDirection: direction, indicator: details.indicator ?? null };
      state._state.write(next);
      const onTabsStateChange = props.onTabsStateChange;
      if (onTabsStateChange !== undefined) onTabsStateChange(next, event);
    });
    return TabsContext.provide(state, () => finishPart(node, props));
  }

  /** Tab-list root. The core attaches its exact arrow/Home/End navigation behavior here. */
  static List(props: PartProps): NativeNode {
    const state = requireContext(TabsContext, "Tabs.List", "Tabs.Root");
    const node = createViewPart(props);
    state.applyTo(node, NativePart.TabsList, undefined);
    return finishPart(node, props);
  }

  /** One controlled tab. Activation, roles, and the panel relationship come from the core. */
  static Tab(props: TabsTabProps): NativeNode {
    const state = requireContext(TabsContext, "Tabs.Tab", "Tabs.Root");
    const node = createButtonPart(props);
    state.applyTo(node, NativePart.Tab, props.value);
    if (props.index !== undefined) setNumber(node, CODE_ITEM_INDEX, props.index);
    setListener(
      node,
      EVENT_CLICK,
      forwardClick(props.onClick, (event: QuickGuiEvent): void => {
        state.select(props.value, event);
      }),
    );
    return TabValueContext.provide(props.value, () => finishPart(node, props));
  }

  /** Decorative indicator mounted by the core only while its tab is active. */
  static Indicator(props: TabsIndicatorProps): NativeNode {
    const state = requireContext(TabsContext, "Tabs.Indicator", "Tabs.Root");
    const inherited = useContext(TabValueContext);
    const node = createViewPart(props);
    const explicit = props.value ?? inherited;
    state.applyTo(node, NativePart.TabIndicator, explicit);
    if (explicit === undefined) {
      createRenderEffect(() => {
        setComponentValue(node, CODE_PART_VALUE, state.value());
      });
    }
    // A declared placement asks the core to keep the indicator anchored to the tab that is
    // really active and to publish that tab's laid-out box back through `useTabsState`.
    if (props.placement !== undefined) setString(node, CODE_ANCHOR_PLACEMENT, props.placement);
    return finishPart(node, props);
  }

  /** One tab panel. The core omits it, or retains it hidden with `keepMounted`, when inactive. */
  static Panel(props: TabsPanelProps): NativeNode {
    const state = requireContext(TabsContext, "Tabs.Panel", "Tabs.Root");
    const node = createViewPart(props);
    state.applyTo(node, NativePart.TabPanel, props.value);
    return finishPart(node, props);
  }
}
