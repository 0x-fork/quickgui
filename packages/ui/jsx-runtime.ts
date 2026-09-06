/**
 * JSX typing for `jsxImportSource: "@quickgui/ui"`.
 *
 * This module is type-only: the QuickGUI compiler lowers JSX to direct component calls before the
 * program is compiled, so nothing here runs. The loosening below lets JSX pass plain values to
 * accessor-typed props and plain nodes to thunk-typed children; the compiler wraps them.
 */

import type { NativeNode } from "@quickgui/native";

type JsxChild = NativeNode | string | number | boolean | null | undefined;

/** A zero-argument accessor prop also accepts the plain value; the compiler wraps it in a thunk. */
type LoosenAccessor<V> = V extends (...args: infer A) => infer R ? (A extends [] ? V | R : V) : V;

/** Thunk- and node-typed children accept anything renderable; the compiler builds the thunk or list. */
type LoosenChildren<V> = V extends (...args: infer A) => infer R
  ? A extends []
    ? V | R | JsxChild | JsxChild[]
    : V
  : V extends NativeNode[] | NativeNode
    ? V | JsxChild | JsxChild[]
    : V;

export namespace JSX {
  export type Element = NativeNode;
  export type ElementType = (props: never) => NativeNode;
  export interface IntrinsicElements {}
  export interface IntrinsicAttributes {}
  export interface ElementChildrenAttribute {
    children: {};
  }
  export type LibraryManagedAttributes<C, P> = {
    [K in keyof P]: K extends "children" ? LoosenChildren<P[K]> : LoosenAccessor<P[K]>;
  };
}
