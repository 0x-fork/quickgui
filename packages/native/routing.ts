import * as binding from "./binding.js";

/** One route declaration consumed and matched by QuickGUI's Rust core. */
export type RouteDefinition = binding.NativeRouteDefinition;
/** One decoded dynamic parameter or query pair. */
export type RouteValue = binding.NativeRouteValue;
/** A normalized internal location returned by the core. */
export type RouteLocation = binding.NativeRouteLocation;
/** The winning leaf route and all of its layout ancestors. */
export type RouteMatch = binding.NativeRouteMatch;
/** A complete immutable-on-read navigation snapshot. */
export type RouterState = binding.NativeRouterState;

/**
 * Core-owned route table and bounded memory history.
 *
 * Every operation is synchronous CPU-only state. It does not access a native window or wait for
 * the main thread; renderer bindings can project the returned snapshots into their own reactive
 * model.
 */
export const Router = binding.NativeRouter;
export type Router = binding.NativeRouter;
