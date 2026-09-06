/**
 * Core-owned route matching and bounded memory history.
 *
 * Every operation is synchronous and CPU-only: the router never reaches the application runtime,
 * so it answers through the host's synchronous service channel.
 */

import { callService } from "./requests.ts";

export interface RouteDefinition {
  id: string;
  /** Omit for a pathless layout route. */
  path?: string;
  parentId?: string;
}

export interface RouteValue {
  name: string;
  value: string;
}

export interface RouteLocation {
  href: string;
  pathname: string;
  search: string;
  hash: string;
  query: RouteValue[];
}

export interface RouteMatch {
  routeIds: string[];
  params: RouteValue[];
}

export interface RouterState {
  location: RouteLocation;
  matched?: RouteMatch;
  historyIndex: number;
  historyLength: number;
  canGoBack: boolean;
  canGoForward: boolean;
}

export class Router {
  readonly id: number;
  #released = false;

  constructor(routes: RouteDefinition[], initialDestination = "/") {
    const json = callService("router-create", JSON.stringify({ routes, initialDestination }));
    this.id = Number(json);
  }

  state(): RouterState {
    return JSON.parse(callService("router-state", JSON.stringify({ id: this.id }))) as RouterState;
  }

  resolve(destination: string): RouteLocation {
    return JSON.parse(callService("router-resolve", JSON.stringify({ id: this.id, destination }))) as RouteLocation;
  }

  isActive(destination: string, end = false): boolean {
    return callService("router-is-active", JSON.stringify({ id: this.id, destination, end })) === "true";
  }

  push(destination: string): RouterState {
    return JSON.parse(callService("router-push", JSON.stringify({ id: this.id, destination }))) as RouterState;
  }

  replace(destination: string): RouterState {
    return JSON.parse(callService("router-replace", JSON.stringify({ id: this.id, destination }))) as RouterState;
  }

  go(delta: number): RouterState {
    return JSON.parse(callService("router-go", JSON.stringify({ id: this.id, delta }))) as RouterState;
  }

  back(): RouterState {
    return JSON.parse(callService("router-back", JSON.stringify({ id: this.id }))) as RouterState;
  }

  forward(): RouterState {
    return JSON.parse(callService("router-forward", JSON.stringify({ id: this.id }))) as RouterState;
  }

  /** Release the core router. Later calls fail. */
  release(): void {
    if (this.#released) return;
    this.#released = true;
    callService("router-release", JSON.stringify({ id: this.id }));
  }
}
