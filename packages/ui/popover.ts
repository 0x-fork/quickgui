/**
 * Popovers anchored to a trigger. `Popover` renders in the window's retained overlay plane;
 * `SystemPopover` renders in a native child window. Both share one controlled open state, and
 * the Rust core owns placement, dismissal, focus, and the hover deadlines.
 */

import {
  NativePart,
  PropertyCode,
  QuickGuiEvent,
  Window,
  type NativeNode,
  type PopoverPlacement,
} from "@quickgui/native";
import { EVENT_CLICK, EVENT_COMPONENT_CHANGE, EVENT_DISMISS } from "@quickgui/native/native-tree";

import {
  CODE_ALIGN,
  CODE_ALIGN_OFFSET,
  CODE_ANCHOR_GAP,
  CODE_ANCHOR_PLACEMENT,
  CODE_CLOSE_DELAY,
  CODE_COLLISION_PADDING,
  CODE_DELAY,
  CODE_DISMISS_ON_ESCAPE,
  CODE_DISMISS_ON_POINTER_OUTSIDE,
  CODE_MODAL,
  CODE_OPEN,
  CODE_OPEN_ON_HOVER,
  CODE_SIDE,
  CODE_SIDE_OFFSET,
  CODE_STICKY,
  CODE_VIEWPORT_MARGIN,
  applyStyle,
} from "./generated.ts";
import { componentChangeFromEvent, type AnchorPlacementDetails } from "./index.ts";
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
import {
  createRenderEffect,
  flush,
  getOwner,
  onCleanup,
  runWithOwner,
  signal,
  untrack,
  type Accessor,
  type Signal,
} from "./reactive.ts";
import {
  Show,
  createRenderer,
  fragment,
  setExplicitBool,
  setListener,
  setMilliseconds,
  setNumber,
  setString,
} from "./runtime.ts";

export type PopoverOpenChangeReason = "trigger-press" | "dismiss" | "hover";

export interface PopoverOpenChangeDetails {
  reason: PopoverOpenChangeReason;
  event: QuickGuiEvent;
}

export type PopoverSide = "top" | "bottom" | "left" | "right";
export type PopoverAlign = "start" | "center" | "end";

/** Positioning declared on `Popover.Positioner`, or on the root as the default. */
export interface AnchorPositioning {
  side: PopoverSide | undefined;
  align: PopoverAlign | undefined;
  sideOffset: number | undefined;
  alignOffset: number | undefined;
  collisionPadding: number | undefined;
  sticky: boolean | undefined;
  /** Anchor to another node instead of the trigger. */
  anchor: NativeNode | undefined;
}

function emptyPositioning(): AnchorPositioning {
  return {
    side: undefined,
    align: undefined,
    sideOffset: undefined,
    alignOffset: undefined,
    collisionPadding: undefined,
    sticky: undefined,
    anchor: undefined,
  };
}

export interface PopoverRootProps {
  children?: () => NativeNode;
  /** Controlled open state. */
  open?: Accessor<boolean>;
  defaultOpen?: boolean;
  onOpenChange?: (open: boolean, details: PopoverOpenChangeDetails) => void;
  dismissOnEscape?: boolean;
  dismissOnPointerOutside?: boolean;
  /** Trap focus in the popup and block pointer input behind it. */
  modal?: boolean;
  /** Open the popup while the trigger is hovered, on the core's own exact deadline. */
  openOnHover?: boolean;
  /** Hover open deadline in milliseconds. Defaults to the core's 300 ms. */
  delay?: number;
  /** Hover close deadline in milliseconds, so the pointer can cross the side offset. */
  closeDelay?: number;
  /** Where the retained tree really placed the popup; style from it like Base UI's `data-side`. */
  onPlacementChange?: (placement: AnchorPlacementDetails, event: QuickGuiEvent) => void;
  /** Default positioning, overridden by whatever `Popover.Positioner` declares. */
  side?: PopoverSide;
  align?: PopoverAlign;
  sideOffset?: number;
  alignOffset?: number;
  collisionPadding?: number;
  sticky?: boolean;
  anchor?: NativeNode;
}

export interface PopoverTriggerProps extends PartProps {
  /** Open the popup on hover instead of on press. */
  openOnHover?: boolean;
  delay?: number;
  closeDelay?: number;
}

export interface PopoverPositionerProps extends PartProps {
  /** Preferred side. The core flips it when the popup does not fit. */
  side?: PopoverSide;
  /** Preferred cross-axis alignment. The core re-aligns it when it does not fit. */
  align?: PopoverAlign;
  /** Distance from the anchor on the placement side, in logical pixels. */
  sideOffset?: number;
  /** Shift along the cross axis, applied before collision handling. */
  alignOffset?: number;
  /** Minimum distance from the viewport edge, in logical pixels. */
  collisionPadding?: number;
  /** Keep the popup inside the viewport. `false` lets it travel with a scrolling anchor. */
  sticky?: boolean;
  /** Anchor to another node. Defaults to the trigger. */
  anchor?: NativeNode;
}

export interface PopoverPopupProps extends PartProps {
  onDismiss?: (event: QuickGuiEvent) => void;
}

export interface PopoverContentProps extends PartProps {
  width: number;
  height: number;
  placement?: PopoverPlacement;
  gap?: number;
  viewportMargin?: number;
}

type PopoverSurface = "popover" | "system-popover";

/** The placement a closed popover reports until the core has placed it once. */
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

/** The state one popover root shares with its parts. */
class PopoverState {
  readonly surface: PopoverSurface;
  readonly scope: string;
  readonly props: PopoverRootProps;
  readonly _open: Signal<boolean>;
  readonly _anchor: Signal<NativeNode | undefined>;
  readonly _placement: Signal<AnchorPlacementDetails>;
  readonly _declared: Signal<AnchorPositioning>;
  readonly _triggers: NativeNode[] = [];

  constructor(surface: PopoverSurface, props: PopoverRootProps) {
    this.surface = surface;
    this.scope = createComponentScope("qg-popover");
    this.props = props;
    this._open = signal<boolean>(props.defaultOpen ?? false);
    this._anchor = signal<NativeNode | undefined>(undefined);
    this._placement = signal<AnchorPlacementDetails>(unresolvedPlacement());
    this._declared = signal<AnchorPositioning>(emptyPositioning());
  }

  open(): boolean {
    const controlled = this.props.open;
    return controlled !== undefined ? controlled() : this._open.read();
  }

  anchor(): NativeNode | undefined {
    return this._anchor.read();
  }

  placement(): AnchorPlacementDetails {
    return this._placement.read();
  }

  /** The positioner's own declaration wins wherever it has one; the root supplies the rest. */
  positioning(): AnchorPositioning {
    const override = this._declared.read();
    const props = this.props;
    return {
      side: override.side ?? props.side,
      align: override.align ?? props.align,
      sideOffset: override.sideOffset ?? props.sideOffset,
      alignOffset: override.alignOffset ?? props.alignOffset,
      collisionPadding: override.collisionPadding ?? props.collisionPadding,
      sticky: override.sticky ?? props.sticky,
      anchor: override.anchor ?? props.anchor,
    };
  }

  declarePositioning(next: AnchorPositioning): void {
    this._declared.write(next);
  }

  dismissOnEscape(): boolean {
    return this.props.dismissOnEscape ?? true;
  }

  dismissOnPointerOutside(): boolean {
    return this.props.dismissOnPointerOutside ?? true;
  }

  registerTrigger(node: NativeNode): void {
    this._triggers.push(node);
    if (untrack(() => this._anchor.read()) === undefined) this._anchor.write(node);
  }

  unregisterTrigger(node: NativeNode): void {
    const index = this._triggers.indexOf(node);
    if (index >= 0) this._triggers.splice(index, 1);
    if (untrack(() => this._anchor.read()) === node) {
      const next = this._triggers.length > 0 ? this._triggers[0] : undefined;
      this._anchor.write(next);
    }
  }

  toggleFromTrigger(node: NativeNode, event: QuickGuiEvent): void {
    this._anchor.write(node);
    this.changeOpen(!this.open(), "trigger-press", event);
  }

  dismiss(event: QuickGuiEvent): void {
    this.changeOpen(false, "dismiss", event);
  }

  /** The core opened or closed the popup itself, on a hover deadline. */
  adoptOpen(next: boolean, event: QuickGuiEvent): void {
    if (next === this.open()) return;
    this.changeOpen(next, "hover", event);
  }

  reportPlacement(next: AnchorPlacementDetails, event: QuickGuiEvent): void {
    this._placement.write(next);
    const onPlacementChange = this.props.onPlacementChange;
    if (onPlacementChange !== undefined) onPlacementChange(next, event);
  }

  changeOpen(next: boolean, reason: PopoverOpenChangeReason, event: QuickGuiEvent): void {
    if (this.props.open === undefined) this._open.write(next);
    const onOpenChange = this.props.onOpenChange;
    if (onOpenChange !== undefined) onOpenChange(next, { reason, event });
  }
}

const PopoverContext = createPartContext<PopoverState>();

function requirePopover(surface: PopoverSurface, part: string): PopoverState {
  const root = surface === "popover" ? "Popover.Root" : "SystemPopover.Root";
  const state = requireContext(PopoverContext, part, root);
  if (state.surface !== surface) throw new TypeError(part + " must be used inside <" + root + ">");
  return state;
}

function createPopoverRoot(surface: PopoverSurface, props: PopoverRootProps): NativeNode {
  const state = new PopoverState(surface, props);
  return PopoverContext.provide(state, () => {
    const children = props.children;
    return children === undefined ? fragment([]) : children();
  });
}

/**
 * Trigger button shared by in-window and system popover roots.
 *
 * For the in-window surface this is the one part the core keeps mounted whether the popover is
 * open or closed, so it carries the whole declaration — the controlled open value, the preferred
 * side and alignment, the offsets, `modal`, and the hover deadlines — and the core reports the
 * placement it really resolved to back through it.
 */
function createTrigger(props: PopoverTriggerProps): NativeNode {
  const state = requireContext(PopoverContext, "Popover.Trigger", "Popover.Root");
  const node = createButtonPart(props);
  if (state.surface === "popover") {
    setPart(node, NativePart.PopoverTrigger, state.scope, undefined);
    createRenderEffect(() => {
      setExplicitBool(node, CODE_OPEN, state.open());
    });
    setExplicitBool(node, CODE_MODAL, state.props.modal);
    setExplicitBool(node, CODE_OPEN_ON_HOVER, props.openOnHover ?? state.props.openOnHover);
    setMilliseconds(node, CODE_DELAY, props.delay ?? state.props.delay);
    setMilliseconds(node, CODE_CLOSE_DELAY, props.closeDelay ?? state.props.closeDelay);
    createRenderEffect(() => {
      const positioning = state.positioning();
      setString(node, CODE_SIDE, positioning.side);
      setString(node, CODE_ALIGN, positioning.align);
      setNumber(node, CODE_SIDE_OFFSET, positioning.sideOffset);
      setNumber(node, CODE_ALIGN_OFFSET, positioning.alignOffset);
      setNumber(node, CODE_COLLISION_PADDING, positioning.collisionPadding);
      setExplicitBool(node, CODE_STICKY, positioning.sticky);
      const anchor = positioning.anchor;
      setString(node, PropertyCode.AnchorTarget, anchor === undefined ? undefined : String(anchor.id));
    });
    setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
      const details = componentChangeFromEvent(event);
      if (details === undefined) return;
      const placement = details.placement;
      if (placement !== undefined) state.reportPlacement(placement, event);
      const open = details.open;
      if (open !== undefined) state.adoptOpen(open, event);
    });
  }
  setListener(
    node,
    EVENT_CLICK,
    forwardClick(props.onClick, (event: QuickGuiEvent): void => {
      state.toggleFromTrigger(node, event);
    }),
  );
  state.registerTrigger(node);
  onCleanup(() => state.unregisterTrigger(node));
  return finishPart(node, props);
}

function createPopoverPart(part: string, props: PartProps, name: string): NativeNode {
  const state = requirePopover("popover", name);
  const node = createViewPart(props);
  setPart(node, part, state.scope, undefined);
  return finishPart(node, props);
}

function createInWindowContent(props: PopoverContentProps, state: PopoverState): NativeNode {
  const anchor = untrack(() => state.anchor());
  if (anchor === undefined) return fragment([]);
  const node = createViewPart(props);
  applyStyle(node, { width: props.width, height: props.height }, undefined);
  setString(node, PropertyCode.AnchorTarget, String(anchor.id));
  setString(node, CODE_ANCHOR_PLACEMENT, props.placement ?? "bottom-start");
  setNumber(node, CODE_ANCHOR_GAP, props.gap ?? 6);
  setNumber(node, CODE_VIEWPORT_MARGIN, props.viewportMargin ?? 8);
  setExplicitBool(node, CODE_DISMISS_ON_ESCAPE, state.dismissOnEscape());
  setExplicitBool(node, CODE_DISMISS_ON_POINTER_OUTSIDE, state.dismissOnPointerOutside());
  setListener(node, EVENT_DISMISS, (event: QuickGuiEvent): void => {
    state.dismiss(event);
  });
  return finishPart(node, props);
}

function createSystemContent(props: PopoverContentProps, state: PopoverState): NativeNode {
  const anchor = untrack(() => state.anchor());
  if (anchor === undefined) return fragment([]);
  const owner = getOwner();
  const placeholder = fragment([]);
  let systemWindow: Window | undefined = undefined;
  let disposing = false;

  // Initial JSX is rendered before its owner Window has a native handle, so the child window is
  // opened from a microtask; later mounts take the same path instead of a special case.
  const open = async (): Promise<void> => {
    await Promise.resolve();
    if (disposing) return;
    const window = new Window({
      title: "QuickGUI System Popover",
      anchor,
      width: props.width,
      height: props.height,
      placement: props.placement ?? "bottom-start",
      gap: props.gap ?? 6,
      viewportMargin: props.viewportMargin ?? 8,
      dismissOnEscape: state.dismissOnEscape(),
      dismissOnPointerOutside: state.dismissOnPointerOutside(),
      renderer: (target: Window) =>
        runWithOwner(owner, () =>
          createRenderer(() => {
            const node = createViewPart(props);
            return finishPart(node, props);
          })(target),
        ),
    });
    window.onClose(() => {
      systemWindow = undefined;
      if (disposing) return;
      // Leave the native close stack before the controlled state unmounts this content.
      const dismissLater = async (): Promise<void> => {
        await Promise.resolve();
        if (disposing) return;
        state.dismiss(new QuickGuiEvent(EVENT_DISMISS, placeholder, undefined));
        flush();
      };
      void dismissLater();
    });
    systemWindow = window;
  };
  void open();

  onCleanup(() => {
    disposing = true;
    const window = systemWindow;
    if (window !== undefined) window.close();
    systemWindow = undefined;
  });
  return placeholder;
}

/** Compound parts for an in-window retained popover. */
export class Popover {
  /** Logical root: it creates no native element of its own. */
  static Root(props: PopoverRootProps): NativeNode {
    return createPopoverRoot("popover", props);
  }

  static Trigger(props: PopoverTriggerProps): NativeNode {
    return createTrigger(props);
  }

  /** Popover content rendered in the current window's retained overlay plane. */
  static Content(props: PopoverContentProps): NativeNode {
    const state = requirePopover("popover", "Popover.Content");
    return Show<NativeNode | undefined>({
      when: () => (state.open() ? state.anchor() : undefined),
      keyed: true,
      children: () => createInWindowContent(props, state),
    });
  }

  /** Portal boundary. QuickGUI's retained overlay node is the portal, so it is the positioner. */
  static Portal(props: PartProps): NativeNode {
    return createPopoverPart(NativePart.PopoverPortal, props, "Popover.Portal");
  }

  /** Pointer-blocking backdrop behind a modal popup. */
  static Backdrop(props: PartProps): NativeNode {
    return createPopoverPart(NativePart.PopoverBackdrop, props, "Popover.Backdrop");
  }

  /** Application-owned positioner; declares the placement props. */
  static Positioner(props: PopoverPositionerProps): NativeNode {
    const state = requirePopover("popover", "Popover.Positioner");
    // The positioner is unmounted while the popover is closed, so its declaration is routed to
    // the trigger, which the core keeps mounted either way.
    state.declarePositioning({
      side: props.side,
      align: props.align,
      sideOffset: props.sideOffset,
      alignOffset: props.alignOffset,
      collisionPadding: props.collisionPadding,
      sticky: props.sticky,
      anchor: props.anchor,
    });
    onCleanup(() => state.declarePositioning(emptyPositioning()));
    const node = createViewPart(props);
    setPart(node, NativePart.PopoverPositioner, state.scope, undefined);
    return finishPart(node, props);
  }

  /** The popup itself, carrying the core's focus containment, restoration, and dismissal. */
  static Popup(props: PopoverPopupProps): NativeNode {
    const state = requirePopover("popover", "Popover.Popup");
    const node = createViewPart(props);
    setPart(node, NativePart.PopoverPopup, state.scope, undefined);
    setExplicitBool(node, CODE_DISMISS_ON_ESCAPE, state.dismissOnEscape());
    setExplicitBool(node, CODE_DISMISS_ON_POINTER_OUTSIDE, state.dismissOnPointerOutside());
    const onDismiss = props.onDismiss;
    setListener(node, EVENT_DISMISS, (event: QuickGuiEvent): void => {
      if (onDismiss !== undefined) onDismiss(event);
      state.dismiss(event);
    });
    return finishPart(node, props);
  }

  /** Arrow pinned to the popup edge that really faces the anchor. */
  static Arrow(props: PartProps): NativeNode {
    return createPopoverPart(NativePart.PopoverArrow, props, "Popover.Arrow");
  }

  /** Scrollable popup body. */
  static Viewport(props: PartProps): NativeNode {
    return createPopoverPart(NativePart.PopoverViewport, props, "Popover.Viewport");
  }

  /** Popup title, which names the popup for assistive technology. */
  static Title(props: PartProps): NativeNode {
    return createPopoverPart(NativePart.PopoverTitle, props, "Popover.Title");
  }

  /** Popup description, which describes the popup for assistive technology. */
  static Description(props: PartProps): NativeNode {
    return createPopoverPart(NativePart.PopoverDescription, props, "Popover.Description");
  }

  /** Close control. The core owns its role and accessible name. */
  static Close(props: PartProps): NativeNode {
    const state = requirePopover("popover", "Popover.Close");
    const node = createButtonPart(props);
    setPart(node, NativePart.PopoverClose, state.scope, undefined);
    setListener(
      node,
      EVENT_CLICK,
      forwardClick(props.onClick, (event: QuickGuiEvent): void => {
        state.dismiss(event);
      }),
    );
    return finishPart(node, props);
  }
}

/** Compound popover parts whose content opens a native child window. */
export class SystemPopover {
  static Root(props: PopoverRootProps): NativeNode {
    return createPopoverRoot("system-popover", props);
  }

  static Trigger(props: PopoverTriggerProps): NativeNode {
    return createTrigger(props);
  }

  /** Popover content rendered through its own renderer in a native child window. */
  static Content(props: PopoverContentProps): NativeNode {
    const state = requirePopover("system-popover", "SystemPopover.Content");
    return Show<NativeNode | undefined>({
      when: () => (state.open() ? state.anchor() : undefined),
      keyed: true,
      children: () => createSystemContent(props, state),
    });
  }
}

/**
 * Read the placement the retained tree really resolved to inside a `Popover.Root` subtree.
 *
 * A declared side and alignment are only a preference: the core flips the side and re-aligns the
 * cross axis whenever the popup does not fit, and publishes the answer during the paint it was
 * already performing.
 */
export function usePopoverPlacement(): Accessor<AnchorPlacementDetails> {
  const state = requirePopover("popover", "usePopoverPlacement");
  return () => state.placement();
}
