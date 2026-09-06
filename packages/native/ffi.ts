/**
 * The C ABI of the QuickGUI host, declared for the scriptc compiler.
 *
 * Every declaration here is signature-only; `ffi.json` binds each one to its `quickgui_*` symbol
 * in the host static library. Calls that reach the platform application thread are
 * fire-and-forget: results and lifecycle outcomes arrive later through the event callback.
 */

/** One host event: `flags` bit 0 marks a present `value`, bit 1 present `extra`, bit 2 present `data`. */
export type HostEventCallback = (
  kind: string,
  window: number,
  target: number,
  flags: number,
  value: string,
  extra: string,
  data: Uint8Array,
) => void;

declare function quickguiProtocolVersion(): number;
declare function quickguiSetEventCallback(onEvent: HostEventCallback): void;
declare function quickguiClearEventCallback(onEvent: HostEventCallback): void;
declare function quickguiCreateApp(options: string): number;
declare function quickguiPrepareApp(app: number, request: number): number;
declare function quickguiAllocateWindow(): number;
declare function quickguiCreateWindow(app: number, window: number, options: string, batch: Uint8Array): number;
declare function quickguiCreateSystemPopover(
  app: number,
  window: number,
  parent: number,
  anchor: number,
  options: string,
  batch: Uint8Array,
): number;
declare function quickguiCreateEmbeddedView(
  app: number,
  window: number,
  parent: number,
  matchHorizontal: number,
  matchVertical: number,
  options: string,
  batch: Uint8Array,
): number;
declare function quickguiApplyBatch(app: number, window: number, batch: Uint8Array): number;
declare function quickguiCloseWindow(app: number, window: number): number;
declare function quickguiFocusNode(app: number, window: number, node: number): number;
declare function quickguiShowDialog(
  app: number,
  window: number,
  request: number,
  kind: number,
  options: string,
): number;
declare function quickguiCommand(app: number, request: number, json: string): number;
declare function quickguiInvoke(request: number, method: string, params: string): number;
declare function quickguiCall(method: string, params: string, reply: (json: string) => void): number;
declare function quickguiDestroyApp(app: number): number;

export function hostProtocolVersion(): number {
  return quickguiProtocolVersion();
}

export function hostSetEventCallback(onEvent: HostEventCallback): void {
  quickguiSetEventCallback(onEvent);
}

export function hostClearEventCallback(onEvent: HostEventCallback): void {
  quickguiClearEventCallback(onEvent);
}

export function hostCreateApp(options: string): number {
  return quickguiCreateApp(options);
}

export function hostPrepareApp(app: number, request: number): void {
  quickguiPrepareApp(app, request);
}

export function hostAllocateWindow(): number {
  return quickguiAllocateWindow();
}

export function hostCreateWindow(app: number, window: number, options: string, batch: Uint8Array): void {
  quickguiCreateWindow(app, window, options, batch);
}

export function hostCreateSystemPopover(
  app: number,
  window: number,
  parent: number,
  anchor: number,
  options: string,
  batch: Uint8Array,
): void {
  quickguiCreateSystemPopover(app, window, parent, anchor, options, batch);
}

export function hostCreateEmbeddedView(
  app: number,
  window: number,
  parent: number,
  matchHorizontal: boolean,
  matchVertical: boolean,
  options: string,
  batch: Uint8Array,
): void {
  quickguiCreateEmbeddedView(app, window, parent, matchHorizontal ? 1 : 0, matchVertical ? 1 : 0, options, batch);
}

export function hostApplyBatch(app: number, window: number, batch: Uint8Array): void {
  quickguiApplyBatch(app, window, batch);
}

export function hostCloseWindow(app: number, window: number): void {
  quickguiCloseWindow(app, window);
}

export function hostFocusNode(app: number, window: number, node: number): void {
  quickguiFocusNode(app, window, node);
}

export function hostShowDialog(app: number, window: number, request: number, kind: number, options: string): void {
  quickguiShowDialog(app, window, request, kind, options);
}

export function hostCommand(app: number, request: number, json: string): void {
  quickguiCommand(app, request, json);
}

export function hostInvoke(request: number, method: string, params: string): void {
  quickguiInvoke(request, method, params);
}

/** Answer one CPU-only service synchronously; the host replies before the call returns. */
export function hostCall(method: string, params: string): string {
  let reply = "";
  quickguiCall(method, params, (json: string) => {
    reply = json;
  });
  return reply;
}

export function hostDestroyApp(app: number): void {
  quickguiDestroyApp(app);
}
