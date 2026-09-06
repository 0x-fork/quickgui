/**
 * Controlled in-window modal dialogs. The Rust core mounts the portal only while the dialog is
 * open and owns focus containment, restoration, dismissal, and the declared transitions.
 */

import { NativePart, type NativeNode, type QuickGuiEvent } from "@quickgui/native";
import { EVENT_CLICK, EVENT_COMPONENT_CHANGE, EVENT_DISMISS } from "@quickgui/native/native-tree";

import {
  CODE_DISMISS_ON_ESCAPE,
  CODE_DISMISS_ON_POINTER_OUTSIDE,
  CODE_ENTER_DURATION,
  CODE_EXIT_DURATION,
  CODE_OPEN,
  CODE_VARIANT,
} from "./generated.ts";
import { componentChangeFromEvent } from "./index.ts";
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
import { createRenderEffect, signal, type Accessor } from "./reactive.ts";
import { fragment, setExplicitBool, setListener, setMilliseconds, setString } from "./runtime.ts";

export type DialogOpenChangeReason = "trigger-press" | "close-press" | "dismiss";

export interface DialogOpenChangeDetails {
  reason: DialogOpenChangeReason;
  event: QuickGuiEvent;
}

export interface DialogRootProps {
  children?: () => NativeNode;
  open?: Accessor<boolean>;
  defaultOpen?: boolean;
  onOpenChange?: (open: boolean, details: DialogOpenChangeDetails) => void;
  /** The core held the surface mounted for the declared exit transition and finished it. */
  onOpenChangeComplete?: (open: boolean, event: QuickGuiEvent) => void;
  dismissOnEscape?: boolean;
  /** Dismiss on a backdrop press. Defaults to `true` for dialogs and `false` for alert dialogs. */
  dismissOnBackdrop?: boolean;
  /** Enter transition the core waits for, in milliseconds. */
  enterDuration?: number;
  /** Exit transition the core keeps the surface mounted for, in milliseconds. */
  exitDuration?: number;
}

export interface DialogPopupProps extends PartProps {
  onDismiss?: (event: QuickGuiEvent) => void;
}

type DialogVariant = "dialog" | "alertdialog";

class DialogState {
  readonly scope: string;
  readonly variant: DialogVariant;
  readonly props: DialogRootProps;
  readonly _open = signal<boolean>(false);

  constructor(variant: DialogVariant, props: DialogRootProps) {
    this.scope = createComponentScope(variant === "alertdialog" ? "qg-alert-dialog" : "qg-dialog");
    this.variant = variant;
    this.props = props;
    this._open.write(props.defaultOpen ?? false);
  }

  open(): boolean {
    const controlled = this.props.open;
    return controlled !== undefined ? controlled() : this._open.read();
  }

  dismissOnEscape(): boolean {
    return this.props.dismissOnEscape ?? true;
  }

  dismissOnBackdrop(): boolean {
    return this.props.dismissOnBackdrop ?? this.variant !== "alertdialog";
  }

  change(next: boolean, reason: DialogOpenChangeReason, event: QuickGuiEvent): void {
    if (this.props.open === undefined) this._open.write(next);
    const onOpenChange = this.props.onOpenChange;
    if (onOpenChange !== undefined) onOpenChange(next, { reason, event });
  }

  complete(next: boolean, event: QuickGuiEvent): void {
    const onOpenChangeComplete = this.props.onOpenChangeComplete;
    if (onOpenChangeComplete !== undefined) onOpenChangeComplete(next, event);
  }

  applyTo(node: NativeNode, part: string): void {
    setPart(node, part, this.scope, undefined);
    setString(node, CODE_VARIANT, this.variant);
    createRenderEffect(() => {
      setExplicitBool(node, CODE_OPEN, this.open());
    });
  }
}

const DialogContext = createPartContext<DialogState>();

function requireDialog(part: string): DialogState {
  return requireContext(DialogContext, part, "Dialog.Root");
}

function createDialogRoot(variant: DialogVariant, props: DialogRootProps): NativeNode {
  const state = new DialogState(variant, props);
  return DialogContext.provide(state, () => {
    const children = props.children;
    return children === undefined ? fragment([]) : children();
  });
}

function createDialogPart(part: string, props: PartProps, name: string): NativeNode {
  const state = requireDialog(name);
  const node = createViewPart(props);
  state.applyTo(node, part);
  return finishPart(node, props);
}

function createTrigger(props: PartProps): NativeNode {
  const state = requireDialog("Dialog.Trigger");
  const node = createButtonPart(props);
  state.applyTo(node, NativePart.DialogTrigger);
  setListener(
    node,
    EVENT_CLICK,
    forwardClick(props.onClick, (event: QuickGuiEvent): void => {
      state.change(true, "trigger-press", event);
    }),
  );
  return finishPart(node, props);
}

/**
 * Viewport portal, focus trap, and focus-restoration boundary. The core mounts it only while the
 * dialog is open, so a closed dialog contributes no overlay, layout, paint, input, or
 * accessibility node.
 */
function createPortal(props: PartProps): NativeNode {
  const state = requireDialog("Dialog.Portal");
  const node = createViewPart(props);
  state.applyTo(node, NativePart.Dialog);
  setMilliseconds(node, CODE_ENTER_DURATION, state.props.enterDuration);
  setMilliseconds(node, CODE_EXIT_DURATION, state.props.exitDuration);
  setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
    const details = componentChangeFromEvent(event);
    if (details === undefined) return;
    const complete = details.openChangeComplete;
    if (complete !== undefined) state.complete(complete, event);
  });
  return finishPart(node, props);
}

/** Modal surface. Escape and backdrop dismissal are decided ahead of time by the core. */
function createPopup(props: DialogPopupProps): NativeNode {
  const state = requireDialog("Dialog.Popup");
  const node = createViewPart(props);
  state.applyTo(node, NativePart.DialogPopup);
  setExplicitBool(node, CODE_DISMISS_ON_ESCAPE, state.dismissOnEscape());
  setExplicitBool(node, CODE_DISMISS_ON_POINTER_OUTSIDE, state.dismissOnBackdrop());
  const onDismiss = props.onDismiss;
  setListener(node, EVENT_DISMISS, (event: QuickGuiEvent): void => {
    if (onDismiss !== undefined) onDismiss(event);
    if (!event.defaultPrevented) state.change(false, "dismiss", event);
  });
  return finishPart(node, props);
}

/** Close control. Its accessible name comes from `ariaLabel`, defaulting to `Close`. */
function createClose(props: PartProps): NativeNode {
  const state = requireDialog("Dialog.Close");
  const node = createButtonPart(props);
  state.applyTo(node, NativePart.DialogClose);
  setListener(
    node,
    EVENT_CLICK,
    forwardClick(props.onClick, (event: QuickGuiEvent): void => {
      state.change(false, "close-press", event);
    }),
  );
  return finishPart(node, props);
}

/** Compound parts for a controlled in-window modal dialog. */
export class Dialog {
  /** Logical root; it creates no native element. */
  static Root(props: DialogRootProps): NativeNode {
    return createDialogRoot("dialog", props);
  }

  /** Trigger button carrying the core's dialog popover and expanded accessibility state. */
  static Trigger(props: PartProps): NativeNode {
    return createTrigger(props);
  }

  static Portal(props: PartProps): NativeNode {
    return createPortal(props);
  }

  /** Scrollable dialog body; a long dialog scrolls inside the popup rather than past the window. */
  static Viewport(props: PartProps): NativeNode {
    return createDialogPart(NativePart.DialogViewport, props, "Dialog.Viewport");
  }

  /** Application-owned backdrop filling the portal. */
  static Backdrop(props: PartProps): NativeNode {
    return createDialogPart(NativePart.DialogBackdrop, props, "Dialog.Backdrop");
  }

  static Popup(props: DialogPopupProps): NativeNode {
    return createPopup(props);
  }

  /** Visible dialog title used as the popup's accessible name. */
  static Title(props: PartProps): NativeNode {
    return createDialogPart(NativePart.DialogTitle, props, "Dialog.Title");
  }

  /** Visible dialog description used as the popup's accessible description. */
  static Description(props: PartProps): NativeNode {
    return createDialogPart(NativePart.DialogDescription, props, "Dialog.Description");
  }

  static Close(props: PartProps): NativeNode {
    return createClose(props);
  }
}

/** Compound parts for a consequential alert dialog whose backdrop does not dismiss by default. */
export class AlertDialog {
  static Root(props: DialogRootProps): NativeNode {
    return createDialogRoot("alertdialog", props);
  }

  static Trigger(props: PartProps): NativeNode {
    return createTrigger(props);
  }

  static Portal(props: PartProps): NativeNode {
    return createPortal(props);
  }

  static Viewport(props: PartProps): NativeNode {
    return createDialogPart(NativePart.DialogViewport, props, "AlertDialog.Viewport");
  }

  static Backdrop(props: PartProps): NativeNode {
    return createDialogPart(NativePart.DialogBackdrop, props, "AlertDialog.Backdrop");
  }

  static Popup(props: DialogPopupProps): NativeNode {
    return createPopup(props);
  }

  static Title(props: PartProps): NativeNode {
    return createDialogPart(NativePart.DialogTitle, props, "AlertDialog.Title");
  }

  static Description(props: PartProps): NativeNode {
    return createDialogPart(NativePart.DialogDescription, props, "AlertDialog.Description");
  }

  static Close(props: PartProps): NativeNode {
    return createClose(props);
  }
}
