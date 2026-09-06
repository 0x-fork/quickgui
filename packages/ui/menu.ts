/**
 * Menus. `Menu` is the Base UI-shaped compound whose rows are ordinary child nodes; `PopoverMenu`
 * and `ContextMenu` declare their rows as one bounded JSON model the Rust core paints itself; and
 * `Menubar` owns which of its menus is open.
 *
 * The core owns validation, highlighting, typeahead, checkbox and radio policy, submenu models,
 * accessibility semantics, and every surface's placement and dismissal. The application never
 * answers a synchronous question while a menu is open.
 */

import { NativePart, PropertyCode, type NativeNode, type PopoverPlacement, type QuickGuiEvent } from "@quickgui/native";
import { EVENT_CLICK, EVENT_COMPONENT_CHANGE, EVENT_DISMISS, EVENT_MENU_SELECT, parseColor } from "@quickgui/native/native-tree";

import {
  CODE_ACTIVE_VALUE,
  CODE_ALIGN,
  CODE_ALIGN_OFFSET,
  CODE_ANCHOR_GAP,
  CODE_ANCHOR_PLACEMENT,
  CODE_ARIA_LABEL,
  CODE_CHECKED,
  CODE_CLOSE_DELAY,
  CODE_CLOSE_ON_CLICK,
  CODE_CLOSE_PARENT_ON_ESC,
  CODE_COLLISION_PADDING,
  CODE_DELAY,
  CODE_DISABLED,
  CODE_DISMISS_ON_ESCAPE,
  CODE_DISMISS_ON_POINTER_OUTSIDE,
  CODE_HREF,
  CODE_ITEM_INDEX,
  CODE_LOOP_FOCUS,
  CODE_MENU,
  CODE_MENU_COUNT,
  CODE_MODAL,
  CODE_OPEN,
  CODE_OPEN_ON_HOVER,
  CODE_ORIENTATION,
  CODE_SIDE,
  CODE_SIDE_OFFSET,
  CODE_STICKY,
  CODE_VIEWPORT_MARGIN,
  applyStyle,
} from "./generated.ts";
import { componentChangeFromEvent, menuSelectionFromEvent, type MenuSelectDetails } from "./index.ts";
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
import { createRenderEffect, onCleanup, signal, untrack, useContext, type Accessor } from "./reactive.ts";
import {
  Show,
  fragment,
  setComponentValue,
  setExplicitBool,
  setListener,
  setMenuLink,
  setMilliseconds,
  setNumber,
  setString,
} from "./runtime.ts";
import type { PopoverOpenChangeReason } from "./popover.ts";

// --- declared menu models ------------------------------------------------------------------------

export type MenuItemKind = "action" | "checkbox" | "radio" | "submenu" | "separator" | "group";

/** One entry of the bounded JSON model a `PopoverMenu` or `ContextMenu` declares. */
export interface MenuItemDeclaration {
  /** Defaults to `submenu` when `items` is present, otherwise `action`. */
  type?: MenuItemKind;
  /** Stable identifier reported by `onSelect`. Required for every interactive entry. */
  id?: string;
  label?: string;
  /** Display-only accelerator hint, such as `"⌘O"`. */
  shortcut?: string;
  /** Radio group key. Defaults to the item's own id. */
  group?: string;
  checked?: boolean;
  disabled?: boolean;
  /** Override the core's default close policy for this entry. */
  closeOnSelect?: boolean;
  /** Alphanumeric navigation label when it differs from the visible one. */
  typeaheadLabel?: string;
  items?: MenuItemDeclaration[];
}

/** Structural geometry and appearance for a declared menu surface. */
export interface MenuAppearance {
  width?: number;
  itemHeight?: number;
  separatorHeight?: number;
  groupLabelHeight?: number;
  verticalPadding?: number;
  fontSize?: number;
  radius?: number;
  padding?: number;
  background?: number | string;
  color?: number | string;
  highlightBackground?: number | string;
  highlightColor?: number | string;
  mutedColor?: number | string;
  /** Wrap arrow navigation at the ends of a menu level. Defaults to `true`. */
  loop?: boolean;
}

/** The declaration the Rust binding decodes: the rows plus packed appearance colours. */
interface EncodedMenu {
  items: MenuItemDeclaration[];
  width?: number | undefined;
  itemHeight?: number | undefined;
  separatorHeight?: number | undefined;
  groupLabelHeight?: number | undefined;
  verticalPadding?: number | undefined;
  fontSize?: number | undefined;
  radius?: number | undefined;
  padding?: number | undefined;
  background?: number | undefined;
  color?: number | undefined;
  highlightBackground?: number | undefined;
  highlightColor?: number | undefined;
  mutedColor?: number | undefined;
  loopFocus?: boolean | undefined;
}

export const MAX_MENU_JSON_BYTES = 256 * 1024;

function packedColor(value: number | string | undefined): number | undefined {
  return value === undefined ? undefined : parseColor(value);
}

/**
 * Encode one bounded menu declaration for the Rust binding.
 *
 * Colours are packed here so the core never parses CSS, and the byte bound is enforced before
 * the declaration crosses the native boundary.
 */
export function encodeMenu(items: MenuItemDeclaration[] | undefined, appearance: MenuAppearance | undefined): string {
  const declaration: EncodedMenu = { items: items ?? [] };
  if (appearance !== undefined) {
    declaration.width = appearance.width;
    declaration.itemHeight = appearance.itemHeight;
    declaration.separatorHeight = appearance.separatorHeight;
    declaration.groupLabelHeight = appearance.groupLabelHeight;
    declaration.verticalPadding = appearance.verticalPadding;
    declaration.fontSize = appearance.fontSize;
    declaration.radius = appearance.radius;
    declaration.padding = appearance.padding;
    declaration.background = packedColor(appearance.background);
    declaration.color = packedColor(appearance.color);
    declaration.highlightBackground = packedColor(appearance.highlightBackground);
    declaration.highlightColor = packedColor(appearance.highlightColor);
    declaration.mutedColor = packedColor(appearance.mutedColor);
    if (appearance.loop !== undefined) declaration.loopFocus = appearance.loop;
  }
  const encoded = JSON.stringify(declaration);
  if (new TextEncoder().encode(encoded).length > MAX_MENU_JSON_BYTES) {
    throw new RangeError("QuickGUI menu declarations are bounded to " + String(MAX_MENU_JSON_BYTES) + " bytes");
  }
  return encoded;
}

// --- popover menu --------------------------------------------------------------------------------

export interface PopoverMenuOpenChangeDetails {
  reason: PopoverOpenChangeReason;
  event: QuickGuiEvent;
}

export interface PopoverMenuRootProps {
  children?: () => NativeNode;
  /** Bounded menu model rebuilt by the Rust core on every declaration change. */
  items?: Accessor<MenuItemDeclaration[]>;
  /** Structural geometry and paint for the declared rows. */
  appearance?: MenuAppearance;
  open?: Accessor<boolean>;
  defaultOpen?: boolean;
  onOpenChange?: (open: boolean, details: PopoverMenuOpenChangeDetails) => void;
  onSelect?: (details: MenuSelectDetails, event: QuickGuiEvent) => void;
  placement?: PopoverPlacement;
  gap?: number;
  viewportMargin?: number;
  dismissOnEscape?: boolean;
  dismissOnPointerOutside?: boolean;
}

export interface PopoverMenuPopupProps extends PartProps {
  /** Surface width. Defaults to the declared appearance width. */
  width?: number;
  onSelect?: (event: QuickGuiEvent) => void;
  onDismiss?: (event: QuickGuiEvent) => void;
}

class PopoverMenuState {
  readonly props: PopoverMenuRootProps;
  readonly _open = signal<boolean>(false);
  readonly _trigger = signal<NativeNode | undefined>(undefined);

  constructor(props: PopoverMenuRootProps) {
    this.props = props;
    this._open.write(props.defaultOpen ?? false);
  }

  open(): boolean {
    const controlled = this.props.open;
    return controlled !== undefined ? controlled() : this._open.read();
  }

  items(): MenuItemDeclaration[] {
    const items = this.props.items;
    return items === undefined ? [] : items();
  }

  menu(): string {
    return encodeMenu(this.items(), this.props.appearance);
  }

  setOpen(next: boolean, reason: PopoverOpenChangeReason, event: QuickGuiEvent): void {
    if (this.props.open === undefined) this._open.write(next);
    const onOpenChange = this.props.onOpenChange;
    if (onOpenChange !== undefined) onOpenChange(next, { reason, event });
  }

  select(details: MenuSelectDetails, event: QuickGuiEvent): void {
    const onSelect = this.props.onSelect;
    if (onSelect !== undefined) onSelect(details, event);
    if (details.submenu === true) return;
    this.setOpen(false, "dismiss", event);
  }
}

const PopoverMenuContext = createPartContext<PopoverMenuState>();

/** Compound parts for a declared popover menu. */
export class PopoverMenu {
  /** Logical root of a declared popover menu. It creates no native element. */
  static Root(props: PopoverMenuRootProps): NativeNode {
    const state = new PopoverMenuState(props);
    return PopoverMenuContext.provide(state, () => {
      const children = props.children;
      return children === undefined ? fragment([]) : children();
    });
  }

  /** Menu trigger. The core supplies its `has-popup`, expansion, and controls relationship. */
  static Trigger(props: PartProps): NativeNode {
    const state = requireContext(PopoverMenuContext, "PopoverMenu.Trigger", "PopoverMenu.Root");
    const node = createButtonPart(props);
    setPart(node, NativePart.PopoverMenuTrigger, undefined, undefined);
    createRenderEffect(() => {
      setExplicitBool(node, CODE_OPEN, state.open());
    });
    setListener(
      node,
      EVENT_CLICK,
      forwardClick(props.onClick, (event: QuickGuiEvent): void => {
        state._trigger.write(node);
        state.setOpen(!state.open(), "trigger-press", event);
      }),
    );
    if (untrack(() => state._trigger.read()) === undefined) state._trigger.write(node);
    return finishPart(node, props);
  }

  /**
   * Menu surface anchored to the trigger.
   *
   * Rows come from the declared model, so the surface owns only its own paint. It is mounted
   * only while the menu is open and while a trigger exists to anchor it.
   */
  static Popup(props: PopoverMenuPopupProps): NativeNode {
    const state = requireContext(PopoverMenuContext, "PopoverMenu.Popup", "PopoverMenu.Root");
    return Show<NativeNode | undefined>({
      when: () => (state.open() ? state._trigger.read() : undefined),
      keyed: true,
      children: () => createPopoverMenuPopup(props, state),
    });
  }
}

function createPopoverMenuPopup(props: PopoverMenuPopupProps, state: PopoverMenuState): NativeNode {
  const anchor = untrack(() => state._trigger.read());
  if (anchor === undefined) return fragment([]);
  const rootProps = state.props;
  const node = createViewPart(props);
  setPart(node, NativePart.PopoverMenuPopup, undefined, undefined);
  setString(node, PropertyCode.AnchorTarget, String(anchor.id));
  createRenderEffect(() => {
    setString(node, CODE_MENU, state.menu());
  });
  const appearance = rootProps.appearance;
  applyStyle(node, { width: props.width ?? (appearance === undefined ? undefined : appearance.width) ?? 224 }, undefined);
  setString(node, CODE_ANCHOR_PLACEMENT, rootProps.placement ?? "bottom-start");
  setNumber(node, CODE_ANCHOR_GAP, rootProps.gap ?? 4);
  setNumber(node, CODE_VIEWPORT_MARGIN, rootProps.viewportMargin ?? 8);
  setExplicitBool(node, CODE_DISMISS_ON_ESCAPE, rootProps.dismissOnEscape ?? true);
  setExplicitBool(node, CODE_DISMISS_ON_POINTER_OUTSIDE, rootProps.dismissOnPointerOutside ?? true);
  const onSelect = props.onSelect;
  setListener(node, EVENT_MENU_SELECT, (event: QuickGuiEvent): void => {
    if (onSelect !== undefined) onSelect(event);
    const details = menuSelectionFromEvent(event);
    if (details !== undefined) state.select(details, event);
  });
  const onDismiss = props.onDismiss;
  setListener(node, EVENT_DISMISS, (event: QuickGuiEvent): void => {
    if (onDismiss !== undefined) onDismiss(event);
    if (!event.defaultPrevented) state.setOpen(false, "dismiss", event);
  });
  return finishPart(node, props);
}

// --- context menu --------------------------------------------------------------------------------

export interface ContextMenuRootProps {
  children?: () => NativeNode;
  items?: Accessor<MenuItemDeclaration[]>;
  appearance?: MenuAppearance;
  onSelect?: (details: MenuSelectDetails, event: QuickGuiEvent) => void;
  /** Wrap the highlight at the ends of a level. Defaults to `true`. */
  loop?: boolean;
}

export interface ContextMenuTriggerProps extends PartProps {
  onSelect?: (event: QuickGuiEvent) => void;
}

class ContextMenuState {
  readonly props: ContextMenuRootProps;

  constructor(props: ContextMenuRootProps) {
    this.props = props;
  }

  menu(): string {
    const items = this.props.items;
    return encodeMenu(items === undefined ? [] : items(), this.props.appearance);
  }
}

const ContextMenuContext = createPartContext<ContextMenuState>();

/** Compound parts for a declared cursor-point context menu. */
export class ContextMenu {
  /** Logical root of a declared context menu. It creates no native element. */
  static Root(props: ContextMenuRootProps): NativeNode {
    const state = new ContextMenuState(props);
    // The Base UI-shaped item parts are the same components here, so the context-menu root also
    // supplies the menu compound context they read; the Rust binding gathers the rows from the
    // trigger's own subtree and the core paints them in its cursor-point surface.
    const compound = new MenuRootState(false, props.loop === undefined ? {} : { loopFocus: props.loop });
    return ContextMenuContext.provide(state, () =>
      MenuContext.provide(compound, () => {
        const children = props.children;
        return children === undefined ? fragment([]) : children();
      }),
    );
  }

  /**
   * Secondary-click target for a declared context menu.
   *
   * The core opens its own cursor-point surface, keeps the menu inside the work area, owns
   * submenu hover intent and safe corridors, and tears the chain down child-first.
   */
  static Trigger(props: ContextMenuTriggerProps): NativeNode {
    const state = requireContext(ContextMenuContext, "ContextMenu.Trigger", "ContextMenu.Root");
    const node = createViewPart(props);
    setPart(node, NativePart.ContextMenuTrigger, undefined, undefined);
    createRenderEffect(() => {
      setString(node, CODE_MENU, state.menu());
    });
    const onSelect = props.onSelect;
    setListener(node, EVENT_MENU_SELECT, (event: QuickGuiEvent): void => {
      if (onSelect !== undefined) onSelect(event);
      const details = menuSelectionFromEvent(event);
      if (details === undefined) return;
      const handler = state.props.onSelect;
      if (handler !== undefined) handler(details, event);
    });
    return finishPart(node, props);
  }
}

// --- Base UI-aligned menu compound ---------------------------------------------------------------

/** Everything the core decided about one menu surface, matching Base UI's popup `data-*`. */
export interface MenuSurfaceState {
  open: boolean;
  side: string;
  align: string;
  anchorHidden: boolean;
}

function settledMenu(): MenuSurfaceState {
  return { open: false, side: "bottom", align: "start", anchorHidden: false };
}

/** Everything the core decided about one menu row, matching Base UI's item `data-*`. */
export interface MenuItemState {
  highlighted: boolean;
  disabled: boolean;
  /** `null` for a row that is not checkable. */
  checked: boolean | null;
  /** Whether this row's submenu is open. */
  open: boolean;
}

function settledMenuItem(): MenuItemState {
  return { highlighted: false, disabled: false, checked: null, open: false };
}

export type MenuSide = "top" | "bottom" | "left" | "right";
export type MenuAlign = "start" | "center" | "end";

export interface MenuPositioning {
  side: MenuSide | undefined;
  align: MenuAlign | undefined;
  sideOffset: number | undefined;
  alignOffset: number | undefined;
  collisionPadding: number | undefined;
  sticky: boolean | undefined;
}

function emptyMenuPositioning(): MenuPositioning {
  return {
    side: undefined,
    align: undefined,
    sideOffset: undefined,
    alignOffset: undefined,
    collisionPadding: undefined,
    sticky: undefined,
  };
}

/** Base UI's `Menu.Root` props. The trigger carries them across the hosted boundary. */
export interface MenuRootProps {
  children?: () => NativeNode;
  open?: Accessor<boolean>;
  defaultOpen?: boolean;
  onOpenChange?: (open: boolean, event: QuickGuiEvent) => void;
  /** Contain Tab focus inside the popup and expect a mounted `Menu.Backdrop`. */
  modal?: boolean;
  /** `"horizontal"` installs the core's horizontal menu key context. */
  orientation?: "horizontal" | "vertical";
  /** Wrap the highlight at the ends of the level. Defaults to `true`. */
  loopFocus?: boolean;
  /** Dismissing this level with Escape closes the level above it too. */
  closeParentOnEsc?: boolean;
  /** Refuse to open at all. */
  disabled?: boolean;
  /** Trigger `openOnHover` default for the whole level. */
  openOnHover?: boolean;
  delay?: number;
  /** Grace period before a hover-opened level closes. Defaults to 100 ms. */
  closeDelay?: number;
  side?: MenuSide;
  align?: MenuAlign;
  sideOffset?: number;
  alignOffset?: number;
  collisionPadding?: number;
  sticky?: boolean;
}

export interface MenuTriggerProps extends PartProps {
  /** Open after `delay` while the pointer rests on the trigger. */
  openOnHover?: boolean;
  delay?: number;
  /** Grace period before a hover-opened level closes. Defaults to 100 ms. */
  closeDelay?: number;
}

export interface MenuPositionerProps extends PartProps {
  side?: MenuSide;
  align?: MenuAlign;
  sideOffset?: number;
  alignOffset?: number;
  collisionPadding?: number;
  sticky?: boolean;
}

/** One declared menu row. */
export interface MenuItemProps extends PartProps {
  /** The row's stable identifier. Required for every interactive row. */
  value?: string;
  /** Accessible name and typeahead label. Defaults to the row's own declared text. */
  label?: string;
  /** Override the core's default close policy for this row. */
  closeOnClick?: boolean;
}

export interface MenuLinkItemProps extends MenuItemProps {
  /** The destination the core hands to its own open-URL path. */
  href?: string;
  /** Reported once the core has opened the destination. */
  onNavigate?: (href: string, event: QuickGuiEvent) => void;
}

export interface MenuCheckboxItemProps extends MenuItemProps {
  checked?: Accessor<boolean>;
  onCheckedChange?: (checked: boolean, event: QuickGuiEvent) => void;
}

export interface MenuRadioItemProps extends MenuItemProps {
  checked?: Accessor<boolean>;
}

export interface MenuRadioGroupProps extends PartProps {
  /** The group's own name. Defaults to one derived from the declaring node. */
  name?: string;
  value?: Accessor<string | undefined>;
  defaultValue?: string;
  onValueChange?: (value: string, event: QuickGuiEvent) => void;
}

export interface MenuGroupLabelProps extends PartProps {
  /** A stable identifier for the label row. */
  value?: string;
  label?: string;
}

export interface MenuSubmenuTriggerProps extends MenuTriggerProps {
  value?: string;
  label?: string;
  closeOnClick?: boolean;
}

class MenuRootState {
  readonly submenu: boolean;
  readonly scope: string;
  readonly props: MenuRootProps;
  readonly _open = signal<boolean>(false);
  readonly _state = signal<MenuSurfaceState>(settledMenu());
  readonly _declared = signal<MenuPositioning>(emptyMenuPositioning());

  constructor(submenu: boolean, props: MenuRootProps) {
    this.submenu = submenu;
    this.scope = createComponentScope("qg-menu");
    this.props = props;
    this._open.write(props.defaultOpen ?? false);
  }

  open(): boolean {
    const controlled = this.props.open;
    return controlled !== undefined ? controlled() : this._open.read();
  }

  state(): MenuSurfaceState {
    return this._state.read();
  }

  setOpen(next: boolean, event: QuickGuiEvent): void {
    if (this.props.open === undefined) this._open.write(next);
    const onOpenChange = this.props.onOpenChange;
    if (onOpenChange !== undefined) onOpenChange(next, event);
  }

  /** The positioner is unmounted while the menu is closed, so its declaration goes to the trigger. */
  positioning(): MenuPositioning {
    const override = this._declared.read();
    const props = this.props;
    return {
      side: override.side ?? props.side,
      align: override.align ?? props.align,
      sideOffset: override.sideOffset ?? props.sideOffset,
      alignOffset: override.alignOffset ?? props.alignOffset,
      collisionPadding: override.collisionPadding ?? props.collisionPadding,
      sticky: override.sticky ?? props.sticky,
    };
  }

  reportSurface(details: { open?: boolean; side?: string; align?: string; anchorHidden?: boolean }): void {
    const current = untrack(() => this._state.read());
    this._state.write({
      open: details.open === true,
      side: details.side ?? current.side,
      align: details.align ?? current.align,
      anchorHidden: details.anchorHidden === true,
    });
  }
}

class MenuItemStateHolder {
  readonly _state = signal<MenuItemState>(settledMenuItem());

  state(): MenuItemState {
    return this._state.read();
  }

  adopt(details: { highlighted?: boolean; disabled?: boolean; checked?: boolean | null; open?: boolean }): void {
    this._state.write({
      highlighted: details.highlighted === true,
      disabled: details.disabled === true,
      checked: details.checked === undefined ? null : details.checked,
      open: details.open === true,
    });
  }
}

class MenuRadioGroupState {
  readonly props: MenuRadioGroupProps;
  readonly _value = signal<string | undefined>(undefined);

  constructor(props: MenuRadioGroupProps) {
    this.props = props;
    this._value.write(props.defaultValue);
  }

  value(): string | undefined {
    const controlled = this.props.value;
    return controlled !== undefined ? controlled() : this._value.read();
  }

  setValue(next: string, event: QuickGuiEvent): void {
    if (this.props.value === undefined) this._value.write(next);
    const onValueChange = this.props.onValueChange;
    if (onValueChange !== undefined) onValueChange(next, event);
  }
}

const MenuContext = createPartContext<MenuRootState>();
const MenuItemContext = createPartContext<MenuItemStateHolder>();
const MenuRadioGroupContext = createPartContext<MenuRadioGroupState>();

function requireMenu(part: string): MenuRootState {
  return requireContext(MenuContext, part, "Menu.Root");
}

/** Read what the core decided about the enclosing menu surface. */
export function useMenuState(): Accessor<MenuSurfaceState> {
  const context = useContext(MenuContext);
  return context === undefined ? settledMenu : () => context.state();
}

/** Read what the core decided about the enclosing menu row. */
export function useMenuItemState(): Accessor<MenuItemState> {
  const context = useContext(MenuItemContext);
  return context === undefined ? settledMenuItem : () => context.state();
}

/** Everything a menu trigger repeats so the Rust binding rebuilds the core descriptor. */
function applyMenuTrigger(node: NativeNode, state: MenuRootState, props: MenuTriggerProps): void {
  const root = state.props;
  createRenderEffect(() => {
    setExplicitBool(node, CODE_OPEN, state.open());
  });
  setExplicitBool(node, CODE_MODAL, root.modal);
  setString(node, CODE_ORIENTATION, root.orientation);
  setExplicitBool(node, CODE_LOOP_FOCUS, root.loopFocus);
  setExplicitBool(node, CODE_CLOSE_PARENT_ON_ESC, root.closeParentOnEsc);
  if (props.disabled === undefined && root.disabled !== undefined) setExplicitBool(node, CODE_DISABLED, root.disabled);
  setExplicitBool(node, CODE_OPEN_ON_HOVER, props.openOnHover ?? root.openOnHover);
  setMilliseconds(node, CODE_DELAY, props.delay ?? root.delay);
  setMilliseconds(node, CODE_CLOSE_DELAY, props.closeDelay ?? root.closeDelay);
  createRenderEffect(() => {
    const positioning = state.positioning();
    setString(node, CODE_SIDE, positioning.side);
    setString(node, CODE_ALIGN, positioning.align);
    setNumber(node, CODE_SIDE_OFFSET, positioning.sideOffset);
    setNumber(node, CODE_ALIGN_OFFSET, positioning.alignOffset);
    setNumber(node, CODE_COLLISION_PADDING, positioning.collisionPadding);
    setExplicitBool(node, CODE_STICKY, positioning.sticky);
  });
}

function adoptSurfaceChange(state: MenuRootState, event: QuickGuiEvent): void {
  const details = componentChangeFromEvent(event);
  if (details === undefined) return;
  state.reportSurface(details);
  const open = details.open;
  if (open !== undefined && open !== state.open()) state.setOpen(open, event);
}

function createMenuPart(name: string, part: string, props: PartProps): NativeNode {
  const state = requireMenu(name);
  const node = createViewPart(props);
  setPart(node, part, state.scope, undefined);
  return finishPart(node, props);
}

/** Shared declaration for every interactive row. */
function createMenuItem(
  name: string,
  part: string,
  props: MenuItemProps,
  declare: (node: NativeNode) => void,
  report: (details: { checked?: boolean | null; href?: string }, event: QuickGuiEvent) => void,
): NativeNode {
  const state = requireMenu(name);
  const holder = new MenuItemStateHolder();
  const node = createViewPart(props);
  setPart(node, part, state.scope, props.value);
  if (props.label !== undefined) setString(node, CODE_ARIA_LABEL, props.label);
  setExplicitBool(node, CODE_CLOSE_ON_CLICK, props.closeOnClick);
  declare(node);
  setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
    const details = componentChangeFromEvent(event);
    if (details === undefined) return;
    holder.adopt(details);
    report(details, event);
  });
  return MenuItemContext.provide(holder, () => finishPart(node, props));
}

function ignoreItemReport(_details: { checked?: boolean | null; href?: string }, _event: QuickGuiEvent): void {}

function declareNothing(_node: NativeNode): void {}

/** Compound parts for an in-window menu whose rows are ordinary child nodes. */
export class Menu {
  /** Logical root of a Base UI-shaped menu. It creates no native element. */
  static Root(props: MenuRootProps): NativeNode {
    const state = new MenuRootState(false, props);
    return MenuContext.provide(state, () => {
      const children = props.children;
      return children === undefined ? fragment([]) : children();
    });
  }

  /** Logical root of one nested menu level. */
  static SubmenuRoot(props: MenuRootProps): NativeNode {
    const state = new MenuRootState(true, props);
    return MenuContext.provide(state, () => {
      const children = props.children;
      return children === undefined ? fragment([]) : children();
    });
  }

  /** Menu trigger. It carries the whole `Menu.Root` declaration the core reads. */
  static Trigger(props: MenuTriggerProps): NativeNode {
    const state = requireMenu("Menu.Trigger");
    const node = createButtonPart(props);
    setPart(node, NativePart.MenuTrigger, state.scope, undefined);
    applyMenuTrigger(node, state, props);
    setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
      adoptSurfaceChange(state, event);
    });
    return finishPart(node, props);
  }

  /**
   * Submenu trigger.
   *
   * It is both a row of its parent level and the trigger of its own, so the core owns its
   * `menuitem` semantics, its `has-popup` relationship, and the expanded state it publishes.
   */
  static SubmenuTrigger(props: MenuSubmenuTriggerProps): NativeNode {
    const state = requireMenu("Menu.SubmenuTrigger");
    const holder = new MenuItemStateHolder();
    const node = createViewPart(props);
    setPart(node, NativePart.MenuSubmenuTrigger, state.scope, props.value ?? state.scope);
    applyMenuTrigger(node, state, props);
    if (props.label !== undefined) setString(node, CODE_ARIA_LABEL, props.label);
    setExplicitBool(node, CODE_CLOSE_ON_CLICK, props.closeOnClick);
    // One node is both a row of its parent level and the trigger of its own, so it reports both
    // the row state the core derived and the surface state its own level resolved to.
    setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
      const details = componentChangeFromEvent(event);
      if (details === undefined) return;
      if (details.highlighted !== undefined) holder.adopt(details);
      if (details.side !== undefined || details.align !== undefined) state.reportSurface(details);
      const open = details.open;
      if (open !== undefined && open !== state.open()) state.setOpen(open, event);
    });
    return MenuItemContext.provide(holder, () => finishPart(node, props));
  }

  /** Portal boundary. QuickGUI's retained overlay node is itself the portal. */
  static Portal(props: PartProps): NativeNode {
    return createMenuPart("Menu.Portal", NativePart.MenuPortal, props);
  }

  /** Application-owned positioner. Base UI declares the placement props here. */
  static Positioner(props: MenuPositionerProps): NativeNode {
    const state = requireMenu("Menu.Positioner");
    state._declared.write({
      side: props.side,
      align: props.align,
      sideOffset: props.sideOffset,
      alignOffset: props.alignOffset,
      collisionPadding: props.collisionPadding,
      sticky: props.sticky,
    });
    onCleanup(() => state._declared.write(emptyMenuPositioning()));
    const node = createViewPart(props);
    setPart(node, NativePart.MenuPositioner, state.scope, undefined);
    return finishPart(node, props);
  }

  /** Full-viewport pointer layer for a modal menu. The core hides it from assistive technology. */
  static Backdrop(props: PartProps): NativeNode {
    return createMenuPart("Menu.Backdrop", NativePart.MenuBackdrop, props);
  }

  /** The menu surface. The core owns its role, dismissal, focus restoration, and key bindings. */
  static Popup(props: PartProps): NativeNode {
    return createMenuPart("Menu.Popup", NativePart.MenuPopup, props);
  }

  /** Caller-owned arrow pinned to the edge the popup really opened against. */
  static Arrow(props: PartProps): NativeNode {
    return createMenuPart("Menu.Arrow", NativePart.MenuArrow, props);
  }

  /** A related group of rows. */
  static Group(props: PartProps): NativeNode {
    return createMenuPart("Menu.Group", NativePart.MenuGroup, props);
  }

  /** A group's visible label row. */
  static GroupLabel(props: MenuGroupLabelProps): NativeNode {
    const state = requireMenu("Menu.GroupLabel");
    const node = createViewPart(props);
    setPart(node, NativePart.MenuGroupLabel, state.scope, props.value);
    if (props.label !== undefined) setString(node, CODE_ARIA_LABEL, props.label);
    return finishPart(node, props);
  }

  /** A non-interactive divider row. */
  static Separator(props: PartProps): NativeNode {
    return createMenuPart("Menu.Separator", NativePart.MenuSeparator, props);
  }

  /** One command row. Activation, closing policy, and semantics come from the core. */
  static Item(props: MenuItemProps): NativeNode {
    return createMenuItem("Menu.Item", NativePart.MenuItem, props, declareNothing, ignoreItemReport);
  }

  /**
   * One link row.
   *
   * QuickGUI has no document to navigate, so activation reaches the platform through the core's
   * own open-URL path and the destination is reported back here.
   */
  static LinkItem(props: MenuLinkItemProps): NativeNode {
    return createMenuItem(
      "Menu.LinkItem",
      NativePart.MenuLinkItem,
      props,
      (node: NativeNode): void => {
        setMenuLink(node, CODE_HREF, props.href);
      },
      (details: { checked?: boolean | null; href?: string }, event: QuickGuiEvent): void => {
        const href = details.href;
        if (href === undefined) return;
        const onNavigate = props.onNavigate;
        if (onNavigate !== undefined) onNavigate(href, event);
      },
    );
  }

  /** One checkbox row. The core owns the toggle and reports the value it committed. */
  static CheckboxItem(props: MenuCheckboxItemProps): NativeNode {
    return createMenuItem(
      "Menu.CheckboxItem",
      NativePart.MenuCheckboxItem,
      props,
      (node: NativeNode): void => {
        const checked = props.checked;
        if (checked !== undefined) {
          createRenderEffect(() => {
            setExplicitBool(node, CODE_CHECKED, checked());
          });
        }
      },
      (details: { checked?: boolean | null; href?: string }, event: QuickGuiEvent): void => {
        const checked = details.checked;
        if (checked === undefined || checked === null) return;
        const onCheckedChange = props.onCheckedChange;
        if (onCheckedChange !== undefined) onCheckedChange(checked, event);
      },
    );
  }

  /** A checkbox row's mark. Mount it only while the row is checked, exactly as Base UI does. */
  static CheckboxItemIndicator(props: PartProps): NativeNode {
    return createMenuPart("Menu.CheckboxItemIndicator", NativePart.MenuCheckboxItemIndicator, props);
  }

  /** A radio group. The core keeps exactly one of its rows checked. */
  static RadioGroup(props: MenuRadioGroupProps): NativeNode {
    const state = requireMenu("Menu.RadioGroup");
    const group = new MenuRadioGroupState(props);
    const node = createViewPart(props);
    setPart(node, NativePart.MenuRadioGroup, state.scope, props.name);
    createRenderEffect(() => {
      setComponentValue(node, CODE_ACTIVE_VALUE, group.value());
    });
    setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
      const details = componentChangeFromEvent(event);
      if (details === undefined) return;
      const value = details.value;
      if (value !== undefined && value !== null) group.setValue(value, event);
    });
    return MenuRadioGroupContext.provide(group, () => finishPart(node, props));
  }

  /** One radio row. */
  static RadioItem(props: MenuRadioItemProps): NativeNode {
    const group = useContext(MenuRadioGroupContext);
    return createMenuItem(
      "Menu.RadioItem",
      NativePart.MenuRadioItem,
      props,
      (node: NativeNode): void => {
        const checked = props.checked;
        const value = props.value;
        createRenderEffect(() => {
          if (checked !== undefined) setExplicitBool(node, CODE_CHECKED, checked());
          else if (group !== undefined && value !== undefined) setExplicitBool(node, CODE_CHECKED, group.value() === value);
        });
      },
      ignoreItemReport,
    );
  }

  /** A radio row's mark. */
  static RadioItemIndicator(props: PartProps): NativeNode {
    return createMenuPart("Menu.RadioItemIndicator", NativePart.MenuRadioItemIndicator, props);
  }
}

// --- menubar -------------------------------------------------------------------------------------

export interface MenubarRootProps extends PartProps {
  /** Number of declared menus. The core clamps it to its own bound. */
  count?: number;
  /** Controlled open menu index; `undefined` for a closed bar. */
  open?: Accessor<number | undefined>;
  defaultOpen?: number;
  onOpenChange?: (open: number | undefined, event: QuickGuiEvent) => void;
  /** The menu that now owns the bar's single Tab stop. */
  onActiveChange?: (index: number, event: QuickGuiEvent) => void;
}

export interface MenubarItemProps extends PartProps {
  /** Position of this menu on the bar. */
  index?: number;
}

/**
 * In-window menubar.
 *
 * The bar owns which menu is open and which one holds its single Tab stop; each menu's surface
 * is an ordinary declared `PopoverMenu` anchored to the matching `Menubar.Item`.
 */
const MenubarContext = createPartContext<string>();

export class Menubar {
  static Root(props: MenubarRootProps): NativeNode {
    const scope = createComponentScope("qg-menubar");
    const uncontrolled = signal<number | undefined>(props.defaultOpen);
    const open = (): number | undefined => {
      const controlled = props.open;
      return controlled !== undefined ? controlled() : uncontrolled.read();
    };
    const node = createViewPart(props);
    setPart(node, NativePart.Menubar, scope, undefined);
    if (props.count !== undefined) setNumber(node, CODE_MENU_COUNT, props.count);
    createRenderEffect(() => {
      const index = open();
      setExplicitBool(node, CODE_OPEN, index !== undefined);
      setNumber(node, CODE_ITEM_INDEX, index);
    });
    setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
      const details = componentChangeFromEvent(event);
      if (details === undefined) return;
      const openIndex = details.openIndex;
      if (openIndex !== undefined) {
        const next = openIndex === null ? undefined : openIndex;
        if (props.open === undefined) uncontrolled.write(next);
        const onOpenChange = props.onOpenChange;
        if (onOpenChange !== undefined) onOpenChange(next, event);
      }
      const focusedIndex = details.focusedIndex;
      if (focusedIndex !== undefined) {
        const onActiveChange = props.onActiveChange;
        if (onActiveChange !== undefined) onActiveChange(focusedIndex, event);
      }
    });
    return MenubarContext.provide(scope, () => finishPart(node, props));
  }

  /** One menubar trigger. Exactly one trigger carries the bar's Tab stop. */
  static Item(props: MenubarItemProps): NativeNode {
    const node = createButtonPart(props);
    setPart(node, NativePart.MenubarItem, useContext(MenubarContext), undefined);
    if (props.index !== undefined) setNumber(node, CODE_ITEM_INDEX, props.index);
    return finishPart(node, props);
  }
}
