/**
 * Fine-grained reactivity: signals, memos, effects, ownership, and context.
 *
 * The graph follows Solid's design. A signal write marks its observers stale and queues them;
 * memos update lazily in dependency order before the effects that read them run, and effects run
 * once per batch. Everything is synchronous and allocation-light: a computation stores its
 * sources and observers in flat arrays with slot indices so unlinking is O(1).
 */

export type Accessor<T> = () => T;
export type Setter<T> = (value: T) => void;

const CLEAN = 0;
const STALE = 1;
const PENDING = 2;

/** Anything a computation can depend on. */
export class Source {
  observers: Computation[] | undefined = undefined;
  observerSlots: number[] | undefined = undefined;
}

/** A disposal scope. Computations are owners too, so nested scopes die with their parent. */
export class Owner extends Source {
  owner: Owner | undefined;
  owned: Owner[] | undefined = undefined;
  cleanups: (() => void)[] | undefined = undefined;
  disposed = false;
  /**
   * The computation whose re-run replaces or removes this scope without owning it: a region's
   * content belongs to the component that created the region, so the render effect that swaps
   * it is not on the content's owner chain. `runTop` runs a stale controller before anything the
   * scope holds, as it runs a stale ancestor, so a computation never runs against state its
   * controller is about to tear down.
   */
  controller: Computation | undefined = undefined;

  constructor(owner: Owner | undefined) {
    super();
    this.owner = owner;
    if (owner !== undefined) {
      if (owner.owned === undefined) owner.owned = [this];
      else owner.owned.push(this);
    }
  }

  /** This owner as a computation, when it is one; dispatch instead of `instanceof` narrowing. */
  asComputation(): Computation | undefined {
    return undefined;
  }
}

export class Computation extends Owner {
  /** The tracked body. Memos replace it after construction. */
  fn: () => void;
  state = CLEAN;
  /** Pure computations (memos) run before effects in every batch. */
  readonly pure: boolean;
  sources: Source[] | undefined = undefined;
  sourceSlots: number[] | undefined = undefined;

  constructor(owner: Owner | undefined, fn: () => void, pure: boolean) {
    super(owner);
    this.fn = fn;
    this.pure = pure;
  }

  asComputation(): Computation | undefined {
    return this;
  }
}

export class Memo<T> extends Computation {
  value: T;
  readonly compute: () => T;
  readonly equals: boolean;

  constructor(owner: Owner | undefined, compute: () => T, equals: boolean) {
    super(owner, noop, true);
    this.compute = compute;
    this.equals = equals;
    this.fn = () => {
      const next = this.compute();
      if (!this.equals || next !== this.value) {
        this.value = next;
        markObservers(this);
      }
    };
    // The field always holds a value of `T`: the first computation runs here, tracked, so the
    // dependencies are linked before anything reads the memo.
    this.value = initialComputation(this, compute);
  }

  read(): T {
    if (this.state !== CLEAN) {
      // Read ahead of the batch: bring this memo up to date in dependency order. The reader is
      // linked afterwards so the refresh never re-queues the computation that is reading.
      if (this.state === PENDING) lookUpstream(this);
      if (this.state === STALE) updateComputation(this);
    }
    if (Listener !== undefined) link(Listener, this);
    return this.value;
  }
}

export class Signal<T> extends Source {
  value: T;
  readonly equals: boolean;

  constructor(value: T, equals: boolean) {
    super();
    this.value = value;
    this.equals = equals;
  }

  read(): T {
    if (Listener !== undefined) link(Listener, this);
    return this.value;
  }

  write(next: T): void {
    if (this.equals && next === this.value) return;
    this.value = next;
    if (this.observers !== undefined && this.observers.length > 0) {
      runUpdates(() => {
        markObservers(this);
      });
    }
  }

  /** Read without tracking. */
  peek(): T {
    return this.value;
  }
}

function noop(): void {}

let Listener: Computation | undefined = undefined;
let CurrentOwner: Owner | undefined = undefined;
let Updates: Computation[] | undefined = undefined;
let Effects: Computation[] | undefined = undefined;

function link(listener: Computation, source: Source): void {
  const sourceSlot = source.observers === undefined ? 0 : source.observers.length;
  let listenerSlot: number;
  if (listener.sources === undefined) {
    listener.sources = [source];
    listener.sourceSlots = [sourceSlot];
    listenerSlot = 0;
  } else {
    listener.sources.push(source);
    listener.sourceSlots!.push(sourceSlot);
    listenerSlot = listener.sources.length - 1;
  }
  if (source.observers === undefined) {
    source.observers = [listener];
    source.observerSlots = [listenerSlot];
  } else {
    source.observers.push(listener);
    source.observerSlots!.push(listenerSlot);
  }
}

/** Mark every observer of a changed source stale and queue it. */
function markObservers(source: Source): void {
  const observers = source.observers;
  if (observers === undefined) return;
  for (const observer of observers) {
    if (observer.state === CLEAN) {
      queue(observer);
      if (observer.observers !== undefined) markDownstream(observer);
    }
    observer.state = STALE;
  }
}

function markDownstream(node: Source): void {
  const observers = node.observers;
  if (observers === undefined) return;
  for (const observer of observers) {
    if (observer.state === CLEAN) {
      observer.state = PENDING;
      queue(observer);
      if (observer.observers !== undefined) markDownstream(observer);
    }
  }
}

function queue(node: Computation): void {
  if (node.pure) {
    if (Updates === undefined) Updates = [node];
    else Updates.push(node);
  } else if (Effects === undefined) {
    Effects = [node];
  } else {
    Effects.push(node);
  }
}

/** Run `fn` inside a batch; queued memos and effects complete when the outermost batch ends. */
function runUpdates(fn: () => void): void {
  if (Updates !== undefined) {
    fn();
    return;
  }
  Updates = [];
  if (Effects === undefined) Effects = [];
  try {
    fn();
    completeUpdates();
  } finally {
    Updates = undefined;
    Effects = undefined;
  }
}

function completeUpdates(): void {
  for (;;) {
    const updates = Updates;
    if (updates !== undefined && updates.length > 0) {
      Updates = [];
      for (const node of updates) runTop(node);
      continue;
    }
    const effects = Effects;
    if (effects !== undefined && effects.length > 0) {
      Effects = [];
      for (const node of effects) runTop(node);
      continue;
    }
    return;
  }
}

/**
 * Run a queued computation after any stale computation ancestor or controller, which would
 * dispose it. Observer order cannot serve here: unlinking swaps the last observer into the freed
 * slot, so after a few updates a computation inside a region may precede the effect that
 * controls the region.
 */
function runTop(node: Computation): void {
  if (node.state === CLEAN || node.disposed) return;
  const ancestors: Computation[] = [];
  let current: Owner | undefined = node.owner;
  while (current !== undefined) {
    const owner: Owner = current;
    if (owner instanceof Computation && owner.state !== CLEAN && !owner.disposed) ancestors.push(owner);
    const controller = owner.controller;
    if (controller !== undefined && controller.state !== CLEAN && !controller.disposed) ancestors.push(controller);
    current = owner.owner;
  }
  let index = ancestors.length;
  while (index > 0) {
    index -= 1;
    const ancestor = ancestors[index]!;
    if (ancestor.state === PENDING) lookUpstream(ancestor);
    if (ancestor.state === STALE) updateComputation(ancestor);
  }
  if (node.disposed) return;
  if (node.state === PENDING) lookUpstream(node);
  if (node.state === STALE) {
    refreshSources(node);
    updateComputation(node);
  }
}

/** Update stale memo sources before a computation runs, so a diamond joins exactly once. */
function refreshSources(node: Computation): void {
  const sources = node.sources;
  if (sources === undefined) return;
  const snapshot = [...sources];
  for (const source of snapshot) {
    if (source instanceof Computation && !source.disposed) {
      if (source.state === PENDING) lookUpstream(source);
      if (source.state === STALE) updateComputation(source);
    }
  }
}

/** Resolve a pending node: update stale memo sources first, then decide whether it changed. */
function lookUpstream(node: Computation): void {
  node.state = CLEAN;
  const sources = node.sources;
  if (sources === undefined) return;
  for (const source of sources) {
    if (source instanceof Computation) {
      if (source.state === STALE) updateComputation(source);
      else if (source.state === PENDING) lookUpstream(source);
      if (node.state === STALE) return;
    }
  }
}

function updateComputation(node: Computation): void {
  if (node.disposed) return;
  cleanNode(node);
  const previousListener = Listener;
  const previousOwner = CurrentOwner;
  Listener = node;
  CurrentOwner = node;
  node.state = CLEAN;
  try {
    node.fn();
  } finally {
    Listener = previousListener;
    CurrentOwner = previousOwner;
  }
}

/** Run a memo's first computation with tracking and return its value. */
function initialComputation<T>(node: Computation, compute: () => T): T {
  const previousListener = Listener;
  const previousOwner = CurrentOwner;
  Listener = node;
  CurrentOwner = node;
  node.state = CLEAN;
  try {
    return compute();
  } finally {
    Listener = previousListener;
    CurrentOwner = previousOwner;
  }
}

/** Unlink a computation from its sources and dispose everything it owns. */
function cleanNode(node: Computation): void {
  const sources = node.sources;
  if (sources !== undefined) {
    const slots = node.sourceSlots!;
    while (sources.length > 0) {
      const source = sources.pop()!;
      const slot = slots.pop()!;
      const observers = source.observers;
      if (observers === undefined || observers.length === 0) continue;
      const last = observers.pop()!;
      const lastSlot = source.observerSlots!.pop()!;
      if (slot < observers.length) {
        // Move the last observer into the freed slot and fix its back-reference.
        last.sourceSlots![lastSlot] = slot;
        observers[slot] = last;
        source.observerSlots![slot] = lastSlot;
      }
    }
  }
  disposeOwned(node);
  runCleanups(node);
}

function disposeOwned(owner: Owner): void {
  const owned = owner.owned;
  if (owned === undefined) return;
  owner.owned = undefined;
  for (const child of owned) {
    if (child instanceof Computation) disposeComputation(child);
    else disposeOwner(child);
  }
}

function disposeComputation(node: Computation): void {
  if (node.disposed) return;
  detachOwner(node);
  node.disposed = true;
  node.state = CLEAN;
  cleanNode(node);
  node.observers = undefined;
  node.observerSlots = undefined;
}

function detachOwner(owner: Owner): void {
  const parent = owner.owner;
  owner.owner = undefined;
  const owned = parent === undefined ? undefined : parent.owned;
  if (owned !== undefined) {
    const index = owned.indexOf(owner);
    if (index >= 0) owned.splice(index, 1);
  }
}

function runCleanups(owner: Owner): void {
  const cleanups = owner.cleanups;
  if (cleanups === undefined) return;
  owner.cleanups = undefined;
  let index = cleanups.length;
  while (index > 0) {
    index -= 1;
    cleanups[index]!();
  }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export interface SignalOptions {
  /** Pass `false` to notify observers on every write, even with an identical value. */
  equals?: boolean;
}

/** A reactive value. Reads inside computations subscribe; writes notify. */
export function createSignal<T>(value: T, options?: SignalOptions): [Accessor<T>, Setter<T>] {
  const signal = new Signal<T>(value, options === undefined ? true : (options.equals ?? true));
  return [() => signal.read(), (next: T) => signal.write(next)];
}

/** A signal exposed as one object, for callers that prefer `count.get()`/`count.set()`. */
export function signal<T>(value: T, options?: SignalOptions): Signal<T> {
  return new Signal<T>(value, options === undefined ? true : (options.equals ?? true));
}

/** A cached derived value that recomputes when its dependencies change. */
export function createMemo<T>(compute: () => T, options?: SignalOptions): Accessor<T> {
  const memo = new Memo<T>(CurrentOwner, compute, options === undefined ? true : (options.equals ?? true));
  return () => memo.read();
}

/** An effect that runs after the current batch and re-runs when its dependencies change. */
export function createEffect(fn: () => void): void {
  const node = new Computation(CurrentOwner, fn, false);
  node.state = STALE;
  if (Updates !== undefined) {
    queue(node);
  } else {
    runUpdates(() => {
      queue(node);
    });
  }
}

/** An effect that runs immediately, for bindings that must be applied during rendering. */
export function createRenderEffect(fn: () => void): void {
  const node = new Computation(CurrentOwner, fn, false);
  updateComputation(node);
}

/** Run `fn` without subscribing the current computation to anything it reads. */
export function untrack<T>(fn: () => T): T {
  const previous = Listener;
  Listener = undefined;
  try {
    return fn();
  } finally {
    Listener = previous;
  }
}

/** Apply several writes and notify once, when the outermost batch ends. */
export function batch(fn: () => void): void {
  runUpdates(fn);
}

/** Create a detached ownership root; `dispose` releases everything created inside `fn`. */
export function createRoot<T>(fn: (dispose: () => void) => T): T {
  const root = new Owner(undefined);
  const dispose = (): void => {
    if (root.disposed) return;
    root.disposed = true;
    disposeOwned(root);
    runCleanups(root);
  };
  const previousOwner = CurrentOwner;
  const previousListener = Listener;
  CurrentOwner = root;
  Listener = undefined;
  try {
    return fn(dispose);
  } finally {
    CurrentOwner = previousOwner;
    Listener = previousListener;
  }
}

export function getOwner(): Owner | undefined {
  return CurrentOwner;
}

/** The computation currently running, if the current owner is one; `untrack` does not change it. */
export function getComputation(): Computation | undefined {
  const owner = CurrentOwner;
  if (owner === undefined) return undefined;
  return owner.asComputation();
}

/** Run `fn` with `owner` as the current owner and no tracking listener. */
export function runWithOwner<T>(owner: Owner | undefined, fn: () => T): T {
  const previousOwner = CurrentOwner;
  const previousListener = Listener;
  CurrentOwner = owner;
  Listener = undefined;
  try {
    return fn();
  } finally {
    CurrentOwner = previousOwner;
    Listener = previousListener;
  }
}

/** Register a cleanup that runs when the current owner is disposed or re-runs. */
export function onCleanup(fn: () => void): void {
  const owner = CurrentOwner;
  if (owner === undefined) return;
  if (owner.cleanups === undefined) owner.cleanups = [fn];
  else owner.cleanups.push(fn);
}

/** Dispose an owner explicitly. Computations dispose their owned scopes when they re-run. */
export function disposeOwner(owner: Owner): void {
  if (owner.disposed) return;
  if (owner instanceof Computation) {
    disposeComputation(owner);
    return;
  }
  detachOwner(owner);
  owner.disposed = true;
  disposeOwned(owner);
  runCleanups(owner);
}

// ---------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------

class ContextEntry<T> {
  readonly owner: Owner;
  readonly value: T;

  constructor(owner: Owner, value: T) {
    this.owner = owner;
    this.value = value;
  }
}

/** A value provided to a subtree and read by descendants through the owner chain. */
export class Context<T> {
  readonly defaultValue: T;
  readonly entries: ContextEntry<T>[] = [];

  constructor(defaultValue: T) {
    this.defaultValue = defaultValue;
  }

  /** Provide `value` to everything created inside `fn`. */
  provide<R>(value: T, fn: () => R): R {
    const owner = new Owner(CurrentOwner);
    const entry = new ContextEntry<T>(owner, value);
    this.entries.push(entry);
    onCleanupOf(owner, () => {
      const index = this.entries.indexOf(entry);
      if (index >= 0) this.entries.splice(index, 1);
    });
    return runWithOwner(owner, fn);
  }

  /** The nearest provided value, or the default. */
  use(): T {
    let current = CurrentOwner;
    while (current !== undefined) {
      const owner: Owner = current;
      for (const entry of this.entries) {
        if (entry.owner === owner) return entry.value;
      }
      current = owner.owner;
    }
    return this.defaultValue;
  }
}

function onCleanupOf(owner: Owner, fn: () => void): void {
  if (owner.cleanups === undefined) owner.cleanups = [fn];
  else owner.cleanups.push(fn);
}

export function createContext<T>(defaultValue: T): Context<T> {
  return new Context<T>(defaultValue);
}

export function useContext<T>(context: Context<T>): T {
  return context.use();
}

/** Run every queued effect now instead of at the end of the enclosing batch. */
export function flush(): void {
  if (Updates === undefined) runUpdates(noop);
}
