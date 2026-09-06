/**
 * Tooltips and preview cards: hover-opened surfaces whose deadlines belong to the Rust core.
 *
 * The compound tooltip sits on ordinary caller-owned elements. `Element.tooltip`, the `tooltip`
 * prop every native node already accepts, stays the shortest path to a native-style hint; this
 * is the composable one, with a shared warm provider, cursor tracking, and a resolved-placement
 * arrow.
 */

import { NativeNodeTag, NativePart, type NativeNode, type PopoverPlacement, type QuickGuiEvent } from "@quickgui/native";
import { EVENT_COMPONENT_CHANGE } from "@quickgui/native/native-tree";

import {
  CODE_ALIGN,
  CODE_ANCHOR_GAP,
  CODE_ANCHOR_PLACEMENT,
  CODE_CLOSE_DELAY,
  CODE_CLOSE_ON_CLICK,
  CODE_COLLISION_PADDING,
  CODE_DELAY,
  CODE_DISABLED,
  CODE_HOVERABLE,
  CODE_OPEN,
  CODE_PROVIDER,
  CODE_SIDE,
  CODE_SIDE_OFFSET,
  CODE_TIMEOUT,
  CODE_TRACK_CURSOR_AXIS,
  CODE_VIEWPORT_MARGIN,
} from "./generated.ts";
import { componentChangeFromEvent, type AnchorPlacementDetails } from "./index.ts";
import {
  createButtonPart,
  createComponentScope,
  createPart,
  createPartContext,
  createViewPart,
  finishPart,
  requireContext,
  setPart,
  type PartProps,
} from "./parts.ts";
import { createRenderEffect, onCleanup, signal, useContext, type Accessor } from "./reactive.ts";
import { fragment, setComponentValue, setExplicitBool, setListener, setMilliseconds, setNumber, setString } from "./runtime.ts";

// --- tooltip -------------------------------------------------------------------------------------

export type TooltipCursorAxis = "none" | "x" | "y" | "both";
export type TooltipSide = "top" | "bottom" | "left" | "right";
export type TooltipAlign = "start" | "center" | "end";

/** The placement a closed tooltip reports until the core has placed it once. */
function unresolvedPlacement(): AnchorPlacementDetails {
  return {
    side: "bottom",
    align: "start",
    anchorHidden: false,
    anchorWidth: 0,
    anchorHeight: 0,
    availableWidth: 0,
    availableHeight: 0,
  };
}

export interface TooltipPositioning {
  side: TooltipSide | undefined;
  align: TooltipAlign | undefined;
  sideOffset: number | undefined;
  collisionPadding: number | undefined;
}

function emptyTooltipPositioning(): TooltipPositioning {
  return { side: undefined, align: undefined, sideOffset: undefined, collisionPadding: undefined };
}

export interface TooltipProviderProps extends PartProps {
  /** Group open deadline in milliseconds. Defaults to the core's 600 ms. */
  delay?: number;
  /** Group close deadline in milliseconds. */
  closeDelay?: number;
  /** How long the group stays warm after the last tooltip closed, in milliseconds. */
  timeout?: number;
}

export interface TooltipRootProps {
  children?: () => NativeNode;
  open?: Accessor<boolean>;
  defaultOpen?: boolean;
  onOpenChange?: (open: boolean, event: QuickGuiEvent) => void;
  /** Cancel any pending deadline and close. The trigger stays focusable. */
  disabled?: boolean;
  /**
   * Let the pointer cross into the popup without closing. Defaults to `false`: a tooltip is a
   * passive help tag like AppKit's, and reaching its popup closes it. Opt in for a popup with
   * content worth hovering.
   */
  hoverable?: boolean;
  /** Follow the cursor on one axis, both, or neither. */
  trackCursorAxis?: TooltipCursorAxis;
  /** Where the core really placed the popup. */
  onPlacementChange?: (placement: AnchorPlacementDetails, event: QuickGuiEvent) => void;
  /** Default positioning, overridden by whatever `Tooltip.Positioner` declares. */
  side?: TooltipSide;
  align?: TooltipAlign;
  sideOffset?: number;
  collisionPadding?: number;
}

export interface TooltipTriggerProps extends PartProps {
  /** Native element this trigger renders. Defaults to a button. */
  element?: "button" | "view" | "text" | "input";
  /** Open deadline in milliseconds, overriding the provider's. */
  delay?: number;
  /** Close deadline in milliseconds, overriding the provider's. */
  closeDelay?: number;
  /** Close on press. Defaults to `true`. */
  closeOnClick?: boolean;
}

export interface TooltipPositionerProps extends PartProps {
  side?: TooltipSide;
  align?: TooltipAlign;
  sideOffset?: number;
  collisionPadding?: number;
}

class TooltipProviderState {
  readonly scope: string;

  constructor() {
    this.scope = createComponentScope("qg-tooltip-provider");
  }
}

class TooltipState {
  readonly scope: string;
  readonly provider: string | undefined;
  readonly props: TooltipRootProps;
  readonly _open = signal<boolean>(false);
  readonly _placement = signal<AnchorPlacementDetails>(unresolvedPlacement());
  readonly _declared = signal<TooltipPositioning>(emptyTooltipPositioning());

  constructor(props: TooltipRootProps, provider: string | undefined) {
    this.scope = createComponentScope("qg-tooltip");
    this.provider = provider;
    this.props = props;
    this._open.write(props.defaultOpen ?? false);
  }

  open(): boolean {
    const controlled = this.props.open;
    return controlled !== undefined ? controlled() : this._open.read();
  }

  placement(): AnchorPlacementDetails {
    return this._placement.read();
  }

  positioning(): TooltipPositioning {
    const override = this._declared.read();
    const props = this.props;
    return {
      side: override.side ?? props.side,
      align: override.align ?? props.align,
      sideOffset: override.sideOffset ?? props.sideOffset,
      collisionPadding: override.collisionPadding ?? props.collisionPadding,
    };
  }

  reportPlacement(next: AnchorPlacementDetails, event: QuickGuiEvent): void {
    this._placement.write(next);
    const onPlacementChange = this.props.onPlacementChange;
    if (onPlacementChange !== undefined) onPlacementChange(next, event);
  }

  adoptOpen(next: boolean, event: QuickGuiEvent): void {
    if (next === this.open()) return;
    if (this.props.open === undefined) this._open.write(next);
    const onOpenChange = this.props.onOpenChange;
    if (onOpenChange !== undefined) onOpenChange(next, event);
  }
}

const TooltipProviderContext = createPartContext<TooltipProviderState>();
const TooltipContext = createPartContext<TooltipState>();

function requireTooltip(part: string): TooltipState {
  return requireContext(TooltipContext, part, "Tooltip.Root");
}

/** Read the placement the core really resolved to inside a `Tooltip.Root` subtree. */
export function useTooltipPlacement(): Accessor<AnchorPlacementDetails> {
  const state = requireTooltip("useTooltipPlacement");
  return () => state.placement();
}

function triggerTag(element: "button" | "view" | "text" | "input" | undefined): number {
  if (element === "view") return NativeNodeTag.View;
  if (element === "text") return NativeNodeTag.Text;
  if (element === "input") return NativeNodeTag.Input;
  return NativeNodeTag.Button;
}

function createTooltipPart(name: string, part: string, props: PartProps): NativeNode {
  const state = requireTooltip(name);
  const node = createViewPart(props);
  setPart(node, part, state.scope, undefined);
  return finishPart(node, props);
}

/** Compound parts for a tooltip. */
export class Tooltip {
  /**
   * Shared warm group.
   *
   * Once one tooltip in the group has opened, an adjacent trigger opens instantly while the
   * group stays warm; that warm window is itself one exact core deadline, so a settled group
   * owns no task or timer. This is one ordinary element, which is also where the group's
   * deadlines are declared.
   */
  static Provider(props: TooltipProviderProps): NativeNode {
    const state = new TooltipProviderState();
    const node = createViewPart(props);
    setPart(node, NativePart.TooltipProvider, state.scope, undefined);
    setMilliseconds(node, CODE_DELAY, props.delay);
    setMilliseconds(node, CODE_CLOSE_DELAY, props.closeDelay);
    setMilliseconds(node, CODE_TIMEOUT, props.timeout);
    return TooltipProviderContext.provide(state, () => finishPart(node, props));
  }

  /** Logical tooltip root. It creates no native element of its own. */
  static Root(props: TooltipRootProps): NativeNode {
    const provider = useContext(TooltipProviderContext);
    const state = new TooltipState(props, provider === undefined ? undefined : provider.scope);
    return TooltipContext.provide(state, () => {
      const children = props.children;
      return children === undefined ? fragment([]) : children();
    });
  }

  /**
   * Tooltip trigger.
   *
   * This is the one part the core keeps mounted whether the tooltip is open or closed, so it
   * carries the whole declaration and reports back what the core decided.
   */
  static Trigger(props: TooltipTriggerProps): NativeNode {
    const state = requireTooltip("Tooltip.Trigger");
    const root = state.props;
    const node = createPart(triggerTag(props.element), props);
    setPart(node, NativePart.TooltipTrigger, state.scope, undefined);
    if (state.provider !== undefined) setComponentValue(node, CODE_PROVIDER, state.provider);
    createRenderEffect(() => {
      setExplicitBool(node, CODE_OPEN, state.open());
    });
    if (props.disabled === undefined && root.disabled !== undefined) setExplicitBool(node, CODE_DISABLED, root.disabled);
    setExplicitBool(node, CODE_HOVERABLE, root.hoverable);
    setString(node, CODE_TRACK_CURSOR_AXIS, root.trackCursorAxis);
    setMilliseconds(node, CODE_DELAY, props.delay);
    setMilliseconds(node, CODE_CLOSE_DELAY, props.closeDelay);
    setExplicitBool(node, CODE_CLOSE_ON_CLICK, props.closeOnClick);
    createRenderEffect(() => {
      const positioning = state.positioning();
      setString(node, CODE_SIDE, positioning.side);
      setString(node, CODE_ALIGN, positioning.align);
      setNumber(node, CODE_SIDE_OFFSET, positioning.sideOffset);
      setNumber(node, CODE_COLLISION_PADDING, positioning.collisionPadding);
    });
    setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
      const details = componentChangeFromEvent(event);
      if (details === undefined) return;
      const placement = details.placement;
      if (placement !== undefined) state.reportPlacement(placement, event);
      const open = details.open;
      if (open !== undefined) state.adoptOpen(open, event);
    });
    return finishPart(node, props);
  }

  /** Tooltip positioner. Base UI declares the placement props here. */
  static Positioner(props: TooltipPositionerProps): NativeNode {
    const state = requireTooltip("Tooltip.Positioner");
    // The positioner is unmounted while the tooltip is closed, so its declaration is routed to
    // the trigger, which the core keeps mounted either way.
    state._declared.write({
      side: props.side,
      align: props.align,
      sideOffset: props.sideOffset,
      collisionPadding: props.collisionPadding,
    });
    onCleanup(() => state._declared.write(emptyTooltipPositioning()));
    const node = createViewPart(props);
    setPart(node, NativePart.TooltipPositioner, state.scope, undefined);
    return finishPart(node, props);
  }

  /** Portal boundary. QuickGUI's retained overlay node is the portal, so it is the positioner. */
  static Portal(props: PartProps): NativeNode {
    return createTooltipPart("Tooltip.Portal", NativePart.TooltipPortal, props);
  }

  /** The tooltip surface. Escape dismissal belongs to the core, so it declares no `onDismiss`. */
  static Popup(props: PartProps): NativeNode {
    return createTooltipPart("Tooltip.Popup", NativePart.TooltipPopup, props);
  }

  /** Arrow pinned to the popup edge that really faces the trigger. */
  static Arrow(props: PartProps): NativeNode {
    return createTooltipPart("Tooltip.Arrow", NativePart.TooltipArrow, props);
  }
}

// --- preview card --------------------------------------------------------------------------------

export interface PreviewCardRootProps extends PartProps {
  open?: Accessor<boolean>;
  defaultOpen?: boolean;
  onOpenChange?: (open: boolean, event: QuickGuiEvent) => void;
  placement?: PopoverPlacement;
  gap?: number;
  viewportMargin?: number;
}

export interface PreviewCardTriggerProps extends PartProps {
  /** Milliseconds the pointer must rest on the trigger before the card opens. */
  delay?: number;
  /** Milliseconds after the pointer leaves both the trigger and the popup. */
  closeDelay?: number;
}

class PreviewCardState {
  readonly scope: string;
  readonly props: PreviewCardRootProps;
  readonly _open = signal<boolean>(false);

  constructor(props: PreviewCardRootProps) {
    this.scope = createComponentScope("qg-preview-card");
    this.props = props;
    this._open.write(props.defaultOpen ?? false);
  }

  open(): boolean {
    const controlled = this.props.open;
    return controlled !== undefined ? controlled() : this._open.read();
  }

  setOpen(next: boolean, event: QuickGuiEvent): void {
    if (this.props.open === undefined) this._open.write(next);
    const onOpenChange = this.props.onOpenChange;
    if (onOpenChange !== undefined) onOpenChange(next, event);
  }
}

const PreviewCardContext = createPartContext<PreviewCardState>();

function createPreviewCardPart(name: string, part: string, props: PartProps): NativeNode {
  const state = requireContext(PreviewCardContext, name, "PreviewCard.Root");
  const node = createViewPart(props);
  setPart(node, part, state.scope, undefined);
  return finishPart(node, props);
}

/** Compound parts for a preview card. */
export class PreviewCard {
  /**
   * Preview-card root owning the two exact deadlines a hover card needs.
   *
   * The core opens after `delay` once the pointer rests on the trigger, closes after
   * `closeDelay` once it has left both the trigger and the popup, and opens immediately on
   * focus.
   */
  static Root(props: PreviewCardRootProps): NativeNode {
    const state = new PreviewCardState(props);
    const node = createViewPart(props);
    setPart(node, NativePart.PreviewCard, state.scope, undefined);
    createRenderEffect(() => {
      setExplicitBool(node, CODE_OPEN, state.open());
    });
    setString(node, CODE_ANCHOR_PLACEMENT, props.placement);
    setNumber(node, CODE_ANCHOR_GAP, props.gap);
    setNumber(node, CODE_VIEWPORT_MARGIN, props.viewportMargin);
    setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
      const details = componentChangeFromEvent(event);
      if (details === undefined) return;
      const open = details.open;
      if (open !== undefined) state.setOpen(open, event);
    });
    return PreviewCardContext.provide(state, () => finishPart(node, props));
  }

  /** Link-like trigger. The core owns the hover deadlines declared here. */
  static Trigger(props: PreviewCardTriggerProps): NativeNode {
    const state = requireContext(PreviewCardContext, "PreviewCard.Trigger", "PreviewCard.Root");
    const node = createButtonPart(props);
    setPart(node, NativePart.PreviewCardTrigger, state.scope, undefined);
    setMilliseconds(node, CODE_DELAY, props.delay);
    setMilliseconds(node, CODE_CLOSE_DELAY, props.closeDelay);
    return finishPart(node, props);
  }

  /** Portal boundary. QuickGUI's retained overlay node is itself the portal. */
  static Portal(props: PartProps): NativeNode {
    return createPreviewCardPart("PreviewCard.Portal", NativePart.PreviewCardPortal, props);
  }

  /** Positioner. Mount either this or the portal, never both. */
  static Positioner(props: PartProps): NativeNode {
    return createPreviewCardPart("PreviewCard.Positioner", NativePart.PreviewCardPositioner, props);
  }

  /** Popup surface. Escape and outside presses dismiss it through the core. */
  static Popup(props: PartProps): NativeNode {
    return createPreviewCardPart("PreviewCard.Popup", NativePart.PreviewCardPopup, props);
  }

  /** Decorative arrow, hidden from assistive technology by the core. */
  static Arrow(props: PartProps): NativeNode {
    return createPreviewCardPart("PreviewCard.Arrow", NativePart.PreviewCardArrow, props);
  }

  /** Optional caller-painted viewport backdrop. */
  static Backdrop(props: PartProps): NativeNode {
    return createPreviewCardPart("PreviewCard.Backdrop", NativePart.PreviewCardBackdrop, props);
  }
}
