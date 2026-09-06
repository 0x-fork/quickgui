/**
 * Base UI parity components: avatar, scroll area, OTP field, and navigation menu.
 *
 * Every one of these declares the Rust core's own compound parts and reads the result back from
 * the asynchronous `componentchange` event; none of them reimplements a delay, a deadline, a
 * focus rule, a mount policy, or a keyboard contract in the application.
 */

import { NativeNodeTag, NativePart, type NativeNode, type PopoverPlacement, type QuickGuiEvent } from "@quickgui/native";
import { EVENT_CLICK, EVENT_COMPONENT_CHANGE } from "@quickgui/native/native-tree";

import {
  CODE_ACTIVE_VALUE,
  CODE_ANCHOR_PLACEMENT,
  CODE_AUTO_SUBMIT,
  CODE_CHECKED,
  CODE_CLOSE_DELAY,
  CODE_CONTENT_SIZE,
  CODE_DELAY,
  CODE_FIT,
  CODE_ITEMS,
  CODE_ITEM_INDEX,
  CODE_KEEP_MOUNTED,
  CODE_LENGTH,
  CODE_LOOP_FOCUS,
  CODE_MASK,
  CODE_OPEN,
  CODE_ORIENTATION,
  CODE_OVERFLOW_EDGE_THRESHOLD,
  CODE_READ_ONLY,
  CODE_REQUIRED,
  CODE_SOURCE,
  CODE_VALUE,
  CODE_VARIANT,
  CODE_VIEWPORT_SIZE,
} from "./generated.ts";
import { componentChangeFromEvent, type ScrollOffset } from "./index.ts";
import {
  applyInputPart,
  createButtonPart,
  createComponentScope,
  createPart,
  createPartContext,
  createViewPart,
  finishPart,
  requireContext,
  setPart,
  type InputPartProps,
  type PartProps,
} from "./parts.ts";
import { createRenderEffect, signal, useContext, type Accessor } from "./reactive.ts";
import { setComponentValue, setExplicitBool, setExtent, setJson, setListener, setMilliseconds, setNumber, setString } from "./runtime.ts";
import type { ComponentItem, Extent } from "./types.ts";

// --- avatar --------------------------------------------------------------------------------------

export interface AvatarRootProps extends PartProps {
  /** Every load-status transition the core decided: `idle`, `loading`, `loaded`, or `error`. */
  onLoadingStatusChange?: (status: string, event: QuickGuiEvent) => void;
}

export interface AvatarImageProps extends PartProps {
  /** Filesystem path, `file://` URL, or a base64 `data:` URL. */
  src: string;
  fit?: "fill" | "contain" | "cover" | "scale-down" | "none";
}

export interface AvatarFallbackProps extends PartProps {
  /** Hold the fallback back for this many milliseconds after loading starts. */
  delay?: number;
}

class AvatarState {
  readonly scope: string;

  constructor() {
    this.scope = createComponentScope("qg-avatar");
  }
}

const AvatarContext = createPartContext<AvatarState>();

/** Compound parts for an avatar. */
export class Avatar {
  /**
   * Avatar root carrying the whole avatar's Image role and accessible name.
   *
   * The core decides which of the image and the fallback is mounted, so swapping between them
   * never changes what assistive technology announces. The binding drives the retained status
   * from the declared image source's own load outcome and reports every transition through
   * `onLoadingStatusChange`.
   */
  static Root(props: AvatarRootProps): NativeNode {
    const state = new AvatarState();
    const node = createViewPart(props);
    setPart(node, NativePart.Avatar, state.scope, undefined);
    const onLoadingStatusChange = props.onLoadingStatusChange;
    if (onLoadingStatusChange !== undefined) {
      setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
        const details = componentChangeFromEvent(event);
        if (details === undefined) return;
        const status = details.loadingStatus;
        if (status !== undefined) onLoadingStatusChange(status, event);
      });
    }
    return AvatarContext.provide(state, () => finishPart(node, props));
  }

  /** Avatar image, mounted by the core only once its declared source has loaded. */
  static Image(props: AvatarImageProps): NativeNode {
    const state = requireContext(AvatarContext, "Avatar.Image", "Avatar.Root");
    const node = createPart(NativeNodeTag.Image, props);
    setPart(node, NativePart.AvatarImage, state.scope, undefined);
    setString(node, CODE_SOURCE, props.src);
    if (props.fit !== undefined) setString(node, CODE_FIT, props.fit);
    return finishPart(node, props);
  }

  /** Avatar fallback, held back for `delay` so a fast decode never flashes initials. */
  static Fallback(props: AvatarFallbackProps): NativeNode {
    const state = requireContext(AvatarContext, "Avatar.Fallback", "Avatar.Root");
    const node = createViewPart(props);
    setPart(node, NativePart.AvatarFallback, state.scope, undefined);
    setMilliseconds(node, CODE_DELAY, props.delay);
    return finishPart(node, props);
  }
}

// --- scroll area ---------------------------------------------------------------------------------

/** The `data-`-like render state one scroll area reports for the application to style from. */
export interface ScrollAreaState {
  /** Offset the core clamped into the scrollable range. */
  offset: ScrollOffset;
  scrolling: boolean;
  hovering: boolean;
  hasOverflowX: boolean;
  hasOverflowY: boolean;
  overflowXStart: boolean;
  overflowXEnd: boolean;
  overflowYStart: boolean;
  overflowYEnd: boolean;
}

function idleScrollArea(): ScrollAreaState {
  return {
    offset: { x: 0, y: 0 },
    scrolling: false,
    hovering: false,
    hasOverflowX: false,
    hasOverflowY: false,
    overflowXStart: false,
    overflowXEnd: false,
    overflowYStart: false,
    overflowYEnd: false,
  };
}

export interface ScrollAreaRootProps extends PartProps {
  /**
   * The viewport extent the application laid out.
   *
   * QuickGUI has no layout observer at the hosted boundary, so the extents the core's
   * arithmetic needs are declared ahead of its decision like every other bounded property.
   */
  viewportSize?: Accessor<Extent>;
  /** The content extent the application laid out. */
  contentSize?: Accessor<Extent>;
  /** Distance from an edge that still counts as being at that edge. Defaults to 1 pixel. */
  overflowEdgeThreshold?: number;
  /** Everything the core decided: the clamped offset and every derived overflow flag. */
  onScrollStateChange?: (state: ScrollAreaState, event: QuickGuiEvent) => void;
}

export interface ScrollAreaScrollbarProps extends PartProps {
  orientation?: "horizontal" | "vertical";
  /**
   * Keep the scrollbar mounted while its axis cannot scroll.
   *
   * The core keeps this per scroll area, so one kept scrollbar keeps the whole area's
   * scrollbars and corner mounted. A mounted-but-useless scrollbar is hidden from assistive
   * technology, so it is never announced.
   */
  keepMounted?: boolean;
}

export interface ScrollAreaThumbProps extends PartProps {
  /** Defaults to the enclosing `ScrollArea.Scrollbar`'s orientation. */
  orientation?: "horizontal" | "vertical";
}

class ScrollAreaState_ {
  readonly scope: string;
  readonly _state = signal<ScrollAreaState>(idleScrollArea());

  constructor() {
    this.scope = createComponentScope("qg-scroll-area");
  }

  state(): ScrollAreaState {
    return this._state.read();
  }
}

class ScrollbarState {
  readonly orientation: "horizontal" | "vertical";

  constructor(orientation: "horizontal" | "vertical") {
    this.orientation = orientation;
  }
}

const ScrollAreaContext = createPartContext<ScrollAreaState_>();
const ScrollbarContext = createPartContext<ScrollbarState>();

/**
 * Read the live scroll state inside a `ScrollArea.Root` subtree.
 *
 * Every flag is the core's own derived render state, so styling a fade, shadow, or scrollbar
 * visibility from it never needs an observer, a timer, or a measurement in the application.
 */
export function useScrollAreaState(): Accessor<ScrollAreaState> {
  const state = requireContext(ScrollAreaContext, "useScrollAreaState", "ScrollArea.Root");
  return () => state.state();
}

function createScrollAreaPart(name: string, part: string, props: PartProps): NativeNode {
  const state = requireContext(ScrollAreaContext, name, "ScrollArea.Root");
  const node = createViewPart(props);
  setPart(node, part, state.scope, undefined);
  return finishPart(node, props);
}

/** Compound parts for a scroll area with caller-drawn scrollbars. */
export class ScrollArea {
  /**
   * Scroll-area root.
   *
   * The core owns the clamped offsets, the derived overflow flags, the thumb arithmetic, and the
   * captured pointer contract; every result comes back through `onScrollStateChange` and
   * `useScrollAreaState`.
   */
  static Root(props: ScrollAreaRootProps): NativeNode {
    const state = new ScrollAreaState_();
    const node = createViewPart(props);
    setPart(node, NativePart.ScrollArea, state.scope, undefined);
    const viewportSize = props.viewportSize;
    if (viewportSize !== undefined) {
      createRenderEffect(() => {
        setExtent(node, CODE_VIEWPORT_SIZE, viewportSize());
      });
    }
    const contentSize = props.contentSize;
    if (contentSize !== undefined) {
      createRenderEffect(() => {
        setExtent(node, CODE_CONTENT_SIZE, contentSize());
      });
    }
    if (props.overflowEdgeThreshold !== undefined) setNumber(node, CODE_OVERFLOW_EDGE_THRESHOLD, props.overflowEdgeThreshold);
    setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
      const details = componentChangeFromEvent(event);
      if (details === undefined) return;
      const offset = details.offset;
      const hasOverflowY = details.hasOverflowY;
      if (offset === undefined || hasOverflowY === undefined) return;
      const next: ScrollAreaState = {
        offset: { x: offset.x, y: offset.y },
        scrolling: details.scrolling === true,
        hovering: details.hovering === true,
        hasOverflowX: details.hasOverflowX === true,
        hasOverflowY,
        overflowXStart: details.overflowXStart === true,
        overflowXEnd: details.overflowXEnd === true,
        overflowYStart: details.overflowYStart === true,
        overflowYEnd: details.overflowYEnd === true,
      };
      state._state.write(next);
      const onScrollStateChange = props.onScrollStateChange;
      if (onScrollStateChange !== undefined) onScrollStateChange(next, event);
    });
    return ScrollAreaContext.provide(state, () => finishPart(node, props));
  }

  /** Clipped viewport. The core answers the wheel and clamps the resulting offset. */
  static Viewport(props: PartProps): NativeNode {
    return createScrollAreaPart("ScrollArea.Viewport", NativePart.ScrollAreaViewport, props);
  }

  /** Scrolled content. Translate it by the negated reported offset. */
  static Content(props: PartProps): NativeNode {
    return createScrollAreaPart("ScrollArea.Content", NativePart.ScrollAreaContent, props);
  }

  /** One caller-drawn scrollbar track, mounted only while its axis can scroll. */
  static Scrollbar(props: ScrollAreaScrollbarProps): NativeNode {
    const state = requireContext(ScrollAreaContext, "ScrollArea.Scrollbar", "ScrollArea.Root");
    const orientation = props.orientation ?? "vertical";
    const node = createViewPart(props);
    setPart(node, NativePart.ScrollAreaScrollbar, state.scope, undefined);
    setString(node, CODE_ORIENTATION, orientation);
    setExplicitBool(node, CODE_KEEP_MOUNTED, props.keepMounted);
    return ScrollbarContext.provide(new ScrollbarState(orientation), () => finishPart(node, props));
  }

  /** One caller-drawn thumb carrying the core's captured drag. */
  static Thumb(props: ScrollAreaThumbProps): NativeNode {
    const state = requireContext(ScrollAreaContext, "ScrollArea.Thumb", "ScrollArea.Root");
    const scrollbar = useContext(ScrollbarContext);
    const node = createViewPart(props);
    setPart(node, NativePart.ScrollAreaThumb, state.scope, undefined);
    setString(node, CODE_ORIENTATION, props.orientation ?? (scrollbar === undefined ? "vertical" : scrollbar.orientation));
    return finishPart(node, props);
  }

  /** The corner between a horizontal and a vertical scrollbar. */
  static Corner(props: PartProps): NativeNode {
    return createScrollAreaPart("ScrollArea.Corner", NativePart.ScrollAreaCorner, props);
  }
}

// --- OTP field -----------------------------------------------------------------------------------

export interface OtpFieldRootProps extends PartProps {
  /** Controlled code. Characters outside the accepted class are dropped by the core. */
  value?: Accessor<string>;
  defaultValue?: string;
  /** Slots retained by the field, bounded by the core's own maximum of twelve. */
  length?: number;
  /** Accepted character class. Defaults to `"numeric"`. */
  validationType?: "numeric" | "alpha" | "alphanumeric" | "none";
  /** Present the code the way a password input is presented. */
  mask?: boolean;
  readOnly?: boolean;
  required?: boolean;
  /** Submit this form through the core as soon as the final slot is filled. */
  autoSubmit?: string;
  onValueChange?: (value: string, event: QuickGuiEvent) => void;
  /** The transition into a full code, which is an edge rather than a value. */
  onComplete?: (value: string, event: QuickGuiEvent) => void;
}

export interface OtpFieldInputProps extends InputPartProps {
  /** The slot this input paints. */
  index: number;
}

export interface OtpFieldSeparatorProps extends PartProps {
  /** Position between two slots, used only to keep the separator's identity stable. */
  index?: number;
}

class OtpFieldState {
  readonly scope: string;

  constructor() {
    this.scope = createComponentScope("qg-otp-field");
  }
}

const OtpFieldContext = createPartContext<OtpFieldState>();

/** Compound parts for an OTP field. */
export class OtpField {
  /**
   * Controlled OTP field.
   *
   * Each slot composes the core's own text input. Accepted characters fill and advance, a paste
   * distributes across consecutive slots, Backspace clears in place and then walks back, and the
   * arrows plus Home and End move between slots, all inside the core.
   */
  static Root(props: OtpFieldRootProps): NativeNode {
    const state = new OtpFieldState();
    const uncontrolled = signal<string>(props.defaultValue ?? "");
    const value = (): string => {
      const controlled = props.value;
      return controlled !== undefined ? controlled() : uncontrolled.read();
    };
    const node = createViewPart(props);
    setPart(node, NativePart.OtpField, state.scope, undefined);
    createRenderEffect(() => {
      setString(node, CODE_VALUE, value());
    });
    if (props.length !== undefined) setNumber(node, CODE_LENGTH, props.length);
    setString(node, CODE_VARIANT, props.validationType);
    setExplicitBool(node, CODE_MASK, props.mask);
    setExplicitBool(node, CODE_READ_ONLY, props.readOnly);
    setExplicitBool(node, CODE_REQUIRED, props.required);
    if (props.autoSubmit !== undefined) setComponentValue(node, CODE_AUTO_SUBMIT, props.autoSubmit);
    setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
      const details = componentChangeFromEvent(event);
      if (details === undefined) return;
      const next = details.value;
      if (next === undefined || next === null) return;
      if (props.value === undefined) uncontrolled.write(next);
      const onValueChange = props.onValueChange;
      if (onValueChange !== undefined) onValueChange(next, event);
      const complete = details.complete;
      if (complete !== undefined) {
        const onComplete = props.onComplete;
        if (onComplete !== undefined) onComplete(complete, event);
      }
    });
    return OtpFieldContext.provide(state, () => finishPart(node, props));
  }

  /** One slot input. The core owns its character, its mask, and every editing key. */
  static Input(props: OtpFieldInputProps): NativeNode {
    const state = requireContext(OtpFieldContext, "OtpField.Input", "OtpField.Root");
    const node = createPart(NativeNodeTag.Input, props);
    applyInputPart(node, props);
    setPart(node, NativePart.OtpFieldInput, state.scope, undefined);
    setNumber(node, CODE_ITEM_INDEX, props.index);
    return finishPart(node, props);
  }

  /** Decorative separator between two slots, hidden from the announced code. */
  static Separator(props: OtpFieldSeparatorProps): NativeNode {
    const state = requireContext(OtpFieldContext, "OtpField.Separator", "OtpField.Root");
    const node = createViewPart(props);
    setPart(node, NativePart.OtpFieldSeparator, state.scope, undefined);
    if (props.index !== undefined) setNumber(node, CODE_ITEM_INDEX, props.index);
    return finishPart(node, props);
  }
}

// --- navigation menu -----------------------------------------------------------------------------

export interface NavigationMenuRootProps extends PartProps {
  /** The open item, or `undefined` while every panel is closed. */
  value?: Accessor<string | undefined>;
  defaultValue?: string;
  onValueChange?: (value: string | undefined, event: QuickGuiEvent) => void;
  orientation?: "horizontal" | "vertical";
  /** Milliseconds a pointer rests on a trigger before its panel opens. Defaults to 50. */
  delay?: number;
  /** Milliseconds after the pointer leaves both a trigger and its popup. Defaults to 50. */
  closeDelay?: number;
  loopFocus?: boolean;
  placement?: PopoverPlacement;
  /**
   * The ordered navigation model.
   *
   * Omit it and the core reads the mounted `NavigationMenu.Item` children in declaration order;
   * declare it to name disabled items or to keep a model the children do not spell out.
   */
  items?: ComponentItem[];
  /** The direction the user's attention travelled, so the application can slide its panel. */
  onActivationDirectionChange?: (direction: string | undefined, event: QuickGuiEvent) => void;
}

export interface NavigationMenuItemProps extends PartProps {
  /** The item's stable value, inherited by every part inside it. */
  value: string;
}

export interface NavigationMenuPartProps extends PartProps {
  /** Defaults to the enclosing `NavigationMenu.Item`'s value. */
  value?: string;
}

export interface NavigationMenuLinkProps extends PartProps {
  /** Stable identity for the link. */
  value: string;
  /** Marks the link for the current destination as the native selected state. */
  active?: boolean;
}

class NavigationMenuState {
  readonly scope: string;

  constructor() {
    this.scope = createComponentScope("qg-navigation-menu");
  }
}

class NavigationMenuItemState {
  readonly value: string;

  constructor(value: string) {
    this.value = value;
  }
}

const NavigationMenuContext = createPartContext<NavigationMenuState>();
const NavigationMenuItemContext = createPartContext<NavigationMenuItemState>();

function navigationMenuPart(name: string, part: string, props: NavigationMenuPartProps, button: boolean): NativeNode {
  const state = requireContext(NavigationMenuContext, name, "NavigationMenu.Root");
  const item = useContext(NavigationMenuItemContext);
  const value = props.value ?? (item === undefined ? undefined : item.value);
  if (value === undefined) throw new TypeError(name + " needs a `value`, or a <NavigationMenu.Item value> ancestor");
  const node = button ? createButtonPart(props) : createViewPart(props);
  setPart(node, part, state.scope, value);
  return finishPart(node, props);
}

/** Compound parts for a navigation menu. */
export class NavigationMenu {
  /**
   * Controlled navigation menu.
   *
   * The core owns the Navigation landmark, the bar's single Tab stop, arrow/Home/End movement
   * with disabled-item skipping, the exact hover open and close deadlines, Escape, and each
   * panel's anchored placement and dismissal.
   */
  static Root(props: NavigationMenuRootProps): NativeNode {
    const state = new NavigationMenuState();
    const uncontrolled = signal<string | undefined>(props.defaultValue);
    const value = (): string | undefined => {
      const controlled = props.value;
      return controlled !== undefined ? controlled() : uncontrolled.read();
    };
    const node = createViewPart(props);
    setPart(node, NativePart.NavigationMenu, state.scope, undefined);
    createRenderEffect(() => {
      setComponentValue(node, CODE_ACTIVE_VALUE, value());
    });
    setString(node, CODE_ORIENTATION, props.orientation);
    setMilliseconds(node, CODE_DELAY, props.delay);
    setMilliseconds(node, CODE_CLOSE_DELAY, props.closeDelay);
    setExplicitBool(node, CODE_LOOP_FOCUS, props.loopFocus);
    setString(node, CODE_ANCHOR_PLACEMENT, props.placement);
    if (props.items !== undefined) setJson(node, CODE_ITEMS, 65536, props.items);
    setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
      const details = componentChangeFromEvent(event);
      if (details === undefined) return;
      if (details.value === undefined) return;
      const next = details.value === null ? undefined : details.value;
      if (next !== value()) {
        if (props.value === undefined) uncontrolled.write(next);
        const onValueChange = props.onValueChange;
        if (onValueChange !== undefined) onValueChange(next, event);
      }
      const onActivationDirectionChange = props.onActivationDirectionChange;
      if (onActivationDirectionChange !== undefined) {
        const direction = details.activationDirection;
        onActivationDirectionChange(direction === undefined || direction === null ? undefined : direction, event);
      }
    });
    return NavigationMenuContext.provide(state, () => finishPart(node, props));
  }

  /** The list of items, carrying the List role and the menu's orientation. */
  static List(props: PartProps): NativeNode {
    const state = requireContext(NavigationMenuContext, "NavigationMenu.List", "NavigationMenu.Root");
    const node = createViewPart(props);
    setPart(node, NativePart.NavigationMenuList, state.scope, undefined);
    return finishPart(node, props);
  }

  /** One item. Its `value` flows to every part inside it. */
  static Item(props: NavigationMenuItemProps): NativeNode {
    const state = requireContext(NavigationMenuContext, "NavigationMenu.Item", "NavigationMenu.Root");
    const node = createViewPart(props);
    setPart(node, NativePart.NavigationMenuItem, state.scope, props.value);
    return NavigationMenuItemContext.provide(new NavigationMenuItemState(props.value), () => finishPart(node, props));
  }

  /** One trigger. Exactly one enabled trigger stays in the window's Tab sequence. */
  static Trigger(props: NavigationMenuPartProps): NativeNode {
    return navigationMenuPart("NavigationMenu.Trigger", NativePart.NavigationMenuTrigger, props, true);
  }

  /** Decorative trigger icon, hidden from the accessible name. */
  static Icon(props: NavigationMenuPartProps): NativeNode {
    return navigationMenuPart("NavigationMenu.Icon", NativePart.NavigationMenuIcon, props, false);
  }

  /** Portal boundary for one item's panel. */
  static Portal(props: NavigationMenuPartProps): NativeNode {
    return navigationMenuPart("NavigationMenu.Portal", NativePart.NavigationMenuPortal, props, false);
  }

  /** Positioner for one item's panel. Mount either this or the portal. */
  static Positioner(props: NavigationMenuPartProps): NativeNode {
    return navigationMenuPart("NavigationMenu.Positioner", NativePart.NavigationMenuPositioner, props, false);
  }

  /** One item's popup surface. */
  static Popup(props: NavigationMenuPartProps): NativeNode {
    return navigationMenuPart("NavigationMenu.Popup", NativePart.NavigationMenuPopup, props, false);
  }

  /** The clipping viewport an application animates a resizing panel inside. */
  static Viewport(props: NavigationMenuPartProps): NativeNode {
    return navigationMenuPart("NavigationMenu.Viewport", NativePart.NavigationMenuViewport, props, false);
  }

  /** One item's panel content, labelled by its trigger. */
  static Content(props: NavigationMenuPartProps): NativeNode {
    return navigationMenuPart("NavigationMenu.Content", NativePart.NavigationMenuContent, props, false);
  }

  /** One item's decorative arrow. */
  static Arrow(props: NavigationMenuPartProps): NativeNode {
    return navigationMenuPart("NavigationMenu.Arrow", NativePart.NavigationMenuArrow, props, false);
  }

  /** One item's optional viewport backdrop. */
  static Backdrop(props: NavigationMenuPartProps): NativeNode {
    return navigationMenuPart("NavigationMenu.Backdrop", NativePart.NavigationMenuBackdrop, props, false);
  }

  /** A navigation link. `active` projects as the native selected state, not a visual class. */
  static Link(props: NavigationMenuLinkProps): NativeNode {
    const state = requireContext(NavigationMenuContext, "NavigationMenu.Link", "NavigationMenu.Root");
    const node = createButtonPart(props);
    setPart(node, NativePart.NavigationMenuLink, state.scope, props.value);
    setExplicitBool(node, CODE_CHECKED, props.active === true);
    return finishPart(node, props);
  }
}
