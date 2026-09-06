/**
 * Toasts. The provider owns the declared queue; the Rust core owns the queue bound, the
 * live-region politeness, the exact auto-dismiss deadline, each toast's stack index, and the
 * swipe arithmetic, and reports all of it back through the viewport.
 */

import { NativePart, type NativeNode, type QuickGuiEvent } from "@quickgui/native";
import { EVENT_COMPONENT_CHANGE } from "@quickgui/native/native-tree";

import { CODE_LIMIT, CODE_PITCH, CODE_STACK_EXPANDED, CODE_SWIPE_DIRECTION, CODE_TIMEOUT, CODE_TOASTS } from "./generated.ts";
import { componentChangeFromEvent, type ToastStackEntry } from "./index.ts";
import {
  createButtonPart,
  createComponentScope,
  createPartContext,
  createViewPart,
  finishPart,
  requireContext,
  setPart,
  type PartProps,
} from "./parts.ts";
import { createRenderEffect, flush, signal, untrack, useContext, type Accessor } from "./reactive.ts";
import { fragment, setExplicitBool, setJson, setListener, setMilliseconds, setNumber, setString } from "./runtime.ts";

export type ToastType = "info" | "success" | "warning" | "error" | "loading";
export type ToastSwipeDirection = "left" | "right" | "up" | "down";

/** One queued toast. Pushing a toast is adding an entry to the declared list. */
export interface ToastDeclaration {
  id: string;
  title: string;
  description?: string | undefined;
  action?: string | undefined;
  /** Base UI's own name for the toast kind. */
  type?: ToastType | undefined;
  /** Auto-dismiss duration in milliseconds. Omit for a toast that stays until dismissed. */
  duration?: number | undefined;
}

/** The fields of a toast an application declares when it pushes one. */
export interface ToastRequest {
  id?: string;
  title: string;
  description?: string;
  action?: string;
  type?: ToastType;
  duration?: number;
}

/** The fields of a queued toast an application can replace in place. */
export interface ToastUpdate {
  title?: string;
  description?: string;
  action?: string;
  type?: ToastType;
  duration?: number;
}

export interface ToastProviderProps {
  children?: () => NativeNode;
  /** Inherited auto-dismiss duration in milliseconds. Defaults to the core's five seconds. */
  timeout?: number;
  /** How many toasts stay unlimited. Older ones are flagged, never silenced. Defaults to 3. */
  limit?: number;
  /** Whether the stack is expanded. */
  expanded?: boolean;
  /** Which way a swipe dismisses a toast. */
  swipeDirection?: ToastSwipeDirection;
  /** Stack pitch in logical pixels, which the core turns into each toast's own offset. */
  pitch?: number;
}

export interface ToastViewportProps extends PartProps {
  /** The bounded queue. Adding an entry pushes a toast; dropping one dismisses it. */
  toasts?: Accessor<ToastDeclaration[]>;
  timeout?: number;
  limit?: number;
  expanded?: boolean;
  swipeDirection?: ToastSwipeDirection;
  pitch?: number;
  /** Every dismissal the core decided, including timed auto-dismissals. */
  onDismiss?: (ids: string[], event: QuickGuiEvent) => void;
  /** Everything the core decided about the queue: index, offset, limited and expanded flags. */
  onStackChange?: (stack: ToastStackEntry[], event: QuickGuiEvent) => void;
}

export interface ToastPartProps extends PartProps {
  /** Declared identifier of the queued toast this part belongs to. */
  toastId: string;
}

let nextToastId = 1;

/**
 * A Base UI-shaped manager over the declared toast list.
 *
 * `add`, `update`, `close`, and `closeAll` change the declared list; the core owns the queue
 * bound, the live-region politeness, the exact auto-dismiss deadline, the stack index each toast
 * is at, and the swipe arithmetic, and reports all of it back through `stack()`.
 */
export class ToastManager {
  readonly scope: string;
  readonly props: ToastProviderProps;
  readonly _queue = signal<ToastDeclaration[]>([]);
  readonly _stack = signal<ToastStackEntry[]>([]);

  constructor(props: ToastProviderProps) {
    this.scope = createComponentScope("qg-toast");
    this.props = props;
  }

  /** The declared queue, which is the source of truth for what is pushed. */
  toasts(): ToastDeclaration[] {
    return this._queue.read();
  }

  /** What the core decided about the queue: stack index, offset, limited and expanded flags. */
  stack(): ToastStackEntry[] {
    return this._stack.read();
  }

  /** Push one toast and return its identifier. */
  add(toast: ToastRequest): string {
    const id = toast.id ?? "qg-toast-" + String(nextToastId);
    nextToastId += 1;
    const entry: ToastDeclaration = {
      id,
      title: toast.title,
      description: toast.description,
      action: toast.action,
      type: toast.type,
      duration: toast.duration,
    };
    const current = untrack(() => this._queue.read());
    this._queue.write([...current, entry]);
    return id;
  }

  /** Replace one queued toast in place, keeping its identity and stack position. */
  update(id: string, toast: ToastUpdate): void {
    const current = untrack(() => this._queue.read());
    const next: ToastDeclaration[] = [];
    for (const entry of current) {
      if (entry.id !== id) {
        next.push(entry);
        continue;
      }
      next.push({
        id,
        title: toast.title ?? entry.title,
        description: toast.description ?? entry.description,
        action: toast.action ?? entry.action,
        type: toast.type ?? entry.type,
        duration: toast.duration ?? entry.duration,
      });
    }
    this._queue.write(next);
  }

  /** Drop one toast from the declaration, which dismisses it. */
  close(id: string): void {
    const current = untrack(() => this._queue.read());
    const next: ToastDeclaration[] = [];
    for (const entry of current) if (entry.id !== id) next.push(entry);
    this._queue.write(next);
  }

  /** Drop every toast. */
  closeAll(): void {
    this._queue.write([]);
  }

  /**
   * Queue a persistent loading toast and turn it into its result.
   *
   * QuickGUI owns no future, so the application drives both halves from the task it already
   * spawned; the toast keeps the same identity and stack position across the transition.
   */
  async promise<T>(work: Promise<T>, loading: string, success: (value: T) => string, failure: (reason: unknown) => string): Promise<T> {
    const id = this.add({ title: loading, type: "loading" });
    flush();
    try {
      const value = await work;
      this.update(id, { title: success(value), type: "success" });
      flush();
      return value;
    } catch (reason) {
      this.update(id, { title: failure(reason), type: "error" });
      flush();
      throw reason;
    }
  }
}

const ToastContext = createPartContext<ToastManager>();

/** Read the toast manager inside a `Toast.Provider` subtree. */
export function useToastManager(): ToastManager {
  return requireContext(ToastContext, "useToastManager", "Toast.Provider");
}

function createToastPart(part: string, props: ToastPartProps, button: boolean): NativeNode {
  const manager = useContext(ToastContext);
  const node = button ? createButtonPart(props) : createViewPart(props);
  setPart(node, part, manager === undefined ? undefined : manager.scope, props.toastId);
  return finishPart(node, props);
}

/** Compound parts for a toast viewport. */
export class Toast {
  /**
   * Toast provider.
   *
   * It owns the declared queue and the provider-level props Base UI puts here: the inherited
   * auto-dismiss `timeout`, the visible stack `limit`, the `expanded` stack, and the swipe
   * contract. It creates no native element of its own.
   */
  static Provider(props: ToastProviderProps): NativeNode {
    const manager = new ToastManager(props);
    return ToastContext.provide(manager, () => {
      const children = props.children;
      return children === undefined ? fragment([]) : children();
    });
  }

  /** Portal boundary above the window's own content. */
  static Portal(props: PartProps): NativeNode {
    const manager = useContext(ToastContext);
    const node = createViewPart(props);
    setPart(node, NativePart.ToastPortal, manager === undefined ? undefined : manager.scope, undefined);
    return finishPart(node, props);
  }

  /**
   * Toast viewport.
   *
   * The queue is a declaration: adding an identifier pushes a toast, dropping one dismisses it,
   * and the core's own bounded queue reports every dismissal, including the timed ones, through
   * `onDismiss`.
   */
  static Viewport(props: ToastViewportProps): NativeNode {
    const manager = useContext(ToastContext);
    const node = createViewPart(props);
    setPart(node, NativePart.ToastViewport, manager === undefined ? undefined : manager.scope, undefined);
    const declared = props.toasts;
    createRenderEffect(() => {
      const toasts = declared !== undefined ? declared() : manager === undefined ? undefined : manager.toasts();
      setJson(node, CODE_TOASTS, 65536, toasts);
    });
    const provider = manager === undefined ? undefined : manager.props;
    setMilliseconds(node, CODE_TIMEOUT, props.timeout ?? (provider === undefined ? undefined : provider.timeout));
    setNumber(node, CODE_LIMIT, props.limit ?? (provider === undefined ? undefined : provider.limit));
    setExplicitBool(node, CODE_STACK_EXPANDED, props.expanded ?? (provider === undefined ? undefined : provider.expanded));
    setString(node, CODE_SWIPE_DIRECTION, props.swipeDirection ?? (provider === undefined ? undefined : provider.swipeDirection));
    setNumber(node, CODE_PITCH, props.pitch ?? (provider === undefined ? undefined : provider.pitch));
    setListener(node, EVENT_COMPONENT_CHANGE, (event: QuickGuiEvent): void => {
      const details = componentChangeFromEvent(event);
      if (details === undefined) return;
      const dismissed = details.dismissed;
      if (dismissed !== undefined) {
        if (manager !== undefined) for (const id of dismissed) manager.close(id);
        const onDismiss = props.onDismiss;
        if (onDismiss !== undefined) onDismiss(dismissed, event);
      }
      const stack = details.toasts;
      if (stack !== undefined) {
        if (manager !== undefined) manager._stack.write(stack);
        const onStackChange = props.onStackChange;
        if (onStackChange !== undefined) onStackChange(stack, event);
      }
    });
    return finishPart(node, props);
  }

  /** One toast positioner, offset by the core's own `index * pitch`. */
  static Positioner(props: ToastPartProps): NativeNode {
    return createToastPart(NativePart.ToastPositioner, props, false);
  }

  /** One queued toast root, projecting the live-region politeness its kind selects. */
  static Root(props: ToastPartProps): NativeNode {
    return createToastPart(NativePart.Toast, props, false);
  }

  /** One toast content box. */
  static Content(props: ToastPartProps): NativeNode {
    return createToastPart(NativePart.ToastContent, props, false);
  }

  /** One toast title, which names the toast for assistive technology. */
  static Title(props: ToastPartProps): NativeNode {
    return createToastPart(NativePart.ToastTitle, props, false);
  }

  /** One toast description, which describes the toast for assistive technology. */
  static Description(props: ToastPartProps): NativeNode {
    return createToastPart(NativePart.ToastDescription, props, false);
  }

  /** One toast action control. */
  static Action(props: ToastPartProps): NativeNode {
    return createToastPart(NativePart.ToastAction, props, true);
  }

  /** One toast close control. Pressing it dismisses the toast through the core's own queue. */
  static Close(props: ToastPartProps): NativeNode {
    return createToastPart(NativePart.ToastClose, props, true);
  }
}
