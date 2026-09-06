/**
 * Routing on top of the core's route table and bounded memory history.
 *
 * Route declarations are static for the lifetime of a `Router`. Location, parameters, query,
 * history traversal, and active-link decisions all come from the Rust core; this module renders
 * the matched route chain and keeps a page mounted while only its parameters or query change.
 */

import {
  Router as NativeRouter,
  type NativeNode,
  type QuickGuiEvent,
  type RouteDefinition,
  type RouteLocation,
  type RouteValue,
  type RouterState,
} from "@quickgui/native";
import { EVENT_CLICK } from "@quickgui/native/native-tree";

import { CODE_ROLE, bindStyleList, type Style } from "./generated.ts";
import {
  applyPartBehavior,
  createPartContext,
  finishPart,
  forwardClick,
  requireContext,
  resolveStyle,
  type PartProps,
} from "./parts.ts";
import { createMemo, createSignal, flush, onCleanup, untrack, useContext, type Accessor, type Setter } from "./reactive.ts";
import { element, fragment, dynamicMaybe, setListener, setString } from "./runtime.ts";
import { NativeNodeTag } from "@quickgui/native";

/** Decoded pattern parameters of the matched route, by name. */
export type RouteParams = Map<string, string>;
/** Decoded query values by name; a repeated name keeps its last value. */
export type RouteSearchParams = Map<string, string>;

/**
 * A route's component. It reads the router through `useParams`, `useLocation`,
 * `useSearchParams`, and `useRouter`, and a layout renders its matched child with `<Outlet />`.
 */
export type RouteComponent = () => NativeNode;

/** One declared route; build these with `route` and `layout`. */
export interface RouteDeclaration {
  /** Stable declaration identity. QuickGUI generates one when omitted. */
  id: string | undefined;
  /** Absolute or parent-relative core route pattern; a pathless layout has none. */
  path: string | undefined;
  component: RouteComponent | undefined;
  children: RouteDeclaration[];
}

/** Declare one route, with its nested routes as the third argument. */
export function route(path: string, component?: RouteComponent, children?: RouteDeclaration[]): RouteDeclaration {
  return { id: undefined, path, component, children: children ?? [] };
}

/** A pathless layout route around `children`. */
export function layout(component: RouteComponent, children: RouteDeclaration[]): RouteDeclaration {
  return { id: undefined, path: undefined, component, children };
}

interface RenderedRoute {
  id: string;
  component: RouteComponent | undefined;
}

interface RouteTable {
  definitions: RouteDefinition[];
  chains: Map<string, RenderedRoute[]>;
}

function buildRouteTable(routes: RouteDeclaration[]): RouteTable {
  const definitions: RouteDefinition[] = [];
  const chains = new Map<string, RenderedRoute[]>();
  let nextId = 1;
  function visit(declaration: RouteDeclaration, parentId: string | undefined, chain: RenderedRoute[]): void {
    let id = declaration.id;
    if (id === undefined) {
      id = "route-" + String(nextId);
      nextId += 1;
    }
    const definition: RouteDefinition = { id };
    if (declaration.path !== undefined) definition.path = declaration.path;
    if (parentId !== undefined) definition.parentId = parentId;
    definitions.push(definition);
    const rendered: RenderedRoute[] = [];
    for (const entry of chain) rendered.push(entry);
    rendered.push({ id, component: declaration.component });
    chains.set(id, rendered);
    for (const child of declaration.children) visit(child, id, rendered);
  }
  for (const declaration of routes) visit(declaration, undefined, []);
  return { definitions, chains };
}

export interface NavigateOptions {
  replace?: boolean;
}

/** Imperative access to the router: the current snapshot and history navigation. */
export class RouterController {
  /** Current core snapshot as an accessor. */
  readonly state: Accessor<RouterState>;
  readonly _native: NativeRouter;
  readonly _setState: Setter<RouterState>;

  constructor(native: NativeRouter, state: Accessor<RouterState>, setState: Setter<RouterState>) {
    this._native = native;
    this.state = state;
    this._setState = setState;
  }

  navigate(destination: string, options?: NavigateOptions): void {
    const replace = options !== undefined && options.replace === true;
    this._commit(replace ? this._native.replace(destination) : this._native.push(destination));
  }

  push(destination: string): void {
    this._commit(this._native.push(destination));
  }

  replace(destination: string): void {
    this._commit(this._native.replace(destination));
  }

  go(delta: number): void {
    this._commit(this._native.go(delta));
  }

  back(): void {
    this._commit(this._native.back());
  }

  forward(): void {
    this._commit(this._native.forward());
  }

  resolve(destination: string): RouteLocation {
    return this._native.resolve(destination);
  }

  /** Whether `destination` is active; `end` requires an exact pathname. Reactive. */
  isActive(destination: string, end?: boolean): boolean {
    this.state();
    return this._native.isActive(destination, end === true);
  }

  _commit(next: RouterState): void {
    this._setState(next);
    // Navigation may happen from an asynchronous task, so close the update boundary here.
    flush();
  }
}

interface RouterContextValue {
  controller: RouterController;
  params: Accessor<RouteParams>;
  searchParams: Accessor<RouteSearchParams>;
}

const RouterContext = createPartContext<RouterContextValue>();
const OutletContext = createPartContext<() => NativeNode>();

export interface RouterProps {
  routes: RouteDeclaration[];
  /** Initial memory-history destination. Defaults to `/`. */
  initialPath?: string;
  /** Rendered when no declared route matches. */
  fallback?: () => NativeNode;
}

function valuesMap(values: RouteValue[]): Map<string, string> {
  const map = new Map<string, string>();
  for (const entry of values) map.set(entry.name, entry.value);
  return map;
}

/** Render the route table's matched chain; declarations never change after creation. */
export function Router(props: RouterProps): NativeNode {
  const table = buildRouteTable(props.routes);
  const native = new NativeRouter(table.definitions, props.initialPath ?? "/");
  onCleanup(() => native.release());
  const [state, setState] = createSignal<RouterState>(native.state());
  const controller = new RouterController(native, state, setState);
  const params = createMemo<RouteParams>(() => {
    const matched = state().matched;
    return valuesMap(matched === undefined ? [] : matched.params);
  });
  const searchParams = createMemo<RouteSearchParams>(() => valuesMap(state().location.query));
  const context: RouterContextValue = { controller, params, searchParams };
  // Equality on this scalar keeps a page mounted while only its params, query, or fragment
  // change. Those values still update through the accessors passed to the component.
  const leaf = createMemo<string | undefined>(() => {
    const matched = state().matched;
    if (matched === undefined) return undefined;
    const ids = matched.routeIds;
    return ids.length === 0 ? undefined : ids[ids.length - 1];
  });
  return RouterContext.provide(context, () =>
    dynamicMaybe(() => {
      const id = leaf();
      if (id === undefined) {
        const fallback = props.fallback;
        return fallback === undefined ? undefined : untrack(fallback);
      }
      const chain = table.chains.get(id);
      if (chain === undefined) throw new Error("QuickGUI core returned unknown route `" + id + "`");
      return untrack(() => renderRouteChain(chain, 0, context));
    }),
  );
}

function renderRouteChain(chain: RenderedRoute[], index: number, context: RouterContextValue): NativeNode {
  const entry = chain[index];
  if (entry === undefined) return fragment([]);
  const outlet = (): NativeNode => renderRouteChain(chain, index + 1, context);
  return OutletContext.provide(outlet, () => {
    const component = entry.component;
    return component === undefined ? outlet() : component();
  });
}

/** Render the next matched child route inside a layout component. */
export function Outlet(): NativeNode {
  const outlet = useContext(OutletContext);
  return outlet === undefined ? fragment([]) : outlet();
}

/** Access the current router. Must be called below `<Router>`. */
export function useRouter(): RouterController {
  return requireContext(RouterContext, "useRouter", "Router").controller;
}

/** Reactive accessor for the normalized current location. */
export function useLocation(): Accessor<RouteLocation> {
  const controller = requireContext(RouterContext, "useLocation", "Router").controller;
  return () => controller.state().location;
}

/** Reactive accessor for decoded parameters from the winning route. */
export function useParams(): Accessor<RouteParams> {
  return requireContext(RouterContext, "useParams", "Router").params;
}

/** Reactive accessor for decoded query values. */
export function useSearchParams(): Accessor<RouteSearchParams> {
  return requireContext(RouterContext, "useSearchParams", "Router").searchParams;
}

/** Reactive accessor for one decoded parameter, or `""` while the winning route has none. */
export function useParam(name: string): Accessor<string> {
  const params = useParams();
  return () => params().get(name) ?? "";
}

/** Reactive accessor for one decoded query value, or `""` while the query has none. */
export function useSearchParam(name: string): Accessor<string> {
  const searchParams = useSearchParams();
  return () => searchParams().get(name) ?? "";
}

/** Stable imperative navigation function for the current router. */
export function useNavigate(): (destination: string, options?: NavigateOptions) => void {
  const controller = requireContext(RouterContext, "useNavigate", "Router").controller;
  return (destination: string, options?: NavigateOptions): void => controller.navigate(destination, options);
}

export interface LinkProps extends PartProps {
  href: string;
  replace?: boolean;
  /** Require an exact pathname for active styling. */
  end?: boolean;
  activeStyle?: Style;
  inactiveStyle?: Style;
}

/** Native link button backed by the current router; `activeStyle` layers over `style` while active. */
export function Link(props: LinkProps): NativeNode {
  const router = useRouter();
  const node = element(NativeNodeTag.Button);
  applyPartBehavior(node, props);
  if (props.role === undefined) setString(node, CODE_ROLE, "link");
  // A style list merges left to right, so the active style layers over the base one and a link
  // remains interactive even when rendered inside a custom title-bar drag region.
  bindStyleList(node, (): Style[] => {
    const styles: Style[] = [];
    const base = resolveStyle(props.style);
    if (base !== undefined) styles.push(base);
    const conditional = router.isActive(props.href, props.end) ? props.activeStyle : props.inactiveStyle;
    if (conditional !== undefined) styles.push(conditional);
    styles.push({ appRegion: "no-drag" });
    return styles;
  });
  setListener(
    node,
    EVENT_CLICK,
    forwardClick(props.onClick, (_event: QuickGuiEvent): void => {
      router.navigate(props.href, props.replace === undefined ? undefined : { replace: props.replace });
    }),
  );
  return finishPart(node, props);
}
