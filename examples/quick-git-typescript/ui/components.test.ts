import { expect, mock, test } from "bun:test";
import { createMemo, createSignal, flush, type Element } from "solid-js";
import type { Store } from "../model/store.ts";
import type { RepositoryStatus } from "../git/status.ts";
import type { NativeNode } from "@quickgui/native";
import { fakeBinding, setEventDispatcher } from "../../../packages/native/test/fake-binding.ts";

mock.module("../../../packages/native/src/binding.ts", () => fakeBinding);
const { Window, PropertyCode, NativeNodeTag, app, parseColor } = await import("@quickgui/native");
const {
  createComponent: nativeComponent,
  createRenderer,
  Text,
  View,
} = await import("@quickgui/solid");
const { AppProvider } = await import("./context.tsx");
const { createStyles, themeFor } = await import("./theme.ts");
const { Toolbar } = await import("./toolbar.tsx");
const { ResizablePanel } = await import("./resize.tsx");
const { NavRow } = await import("./sidebar.tsx");
setEventDispatcher(() => app.dispatchEvents());
await app.whenReady();

const createComponent = <P>(component: (props: P) => Element, props: P): NativeNode =>
  nativeComponent(component as (props: P) => NativeNode, props);

function mount(component: () => NativeNode, store: Partial<Store> = {}) {
  const theme = () => themeFor("light");
  const window = new Window({
    renderer: createRenderer(() =>
      createComponent(AppProvider, {
        value: {
          store: store as Store,
          window: Window.getCurrentWindow(),
          theme,
          styles: () => createStyles(theme()),
          dialog: () => undefined,
          openDialog: () => {},
          closeDialog: () => {},
          openRepository: async () => {},
          openRepositoryPath: async () => {},
        },
        get children() {
          return component();
        },
      }),
    ),
  });
  flush();
  return window;
}
function labeled(window: InstanceType<typeof Window>, label: string) {
  const node = [...window.nodes.values()].find(
    (node) => node.properties.get(PropertyCode.AccessibilityLabel) === label,
  );
  if (!node) throw new Error(`Missing ${label}`);
  return node;
}

test("toolbar retains QuickGUI buttons while task state and upstream counts change", () => {
  const [status, setStatus] = createSignal<RepositoryStatus>({
    branch: "main",
    upstream: "origin/main",
    ahead: 2,
    behind: 1,
    hasUpstreamCounts: true,
    detached: false,
    stashCount: 0,
    entries: [],
  });
  const [busy, setBusy] = createSignal<ReturnType<Store["busy"]>>();
  let fetches = 0;
  let cancelled = false;
  const window = mount(() => createComponent(Toolbar, {}), {
    status,
    busy,
    conflicts: createMemo(() => 0),
    setView: () => {},
    refresh: async () => {},
    fetch: async () => {
      fetches++;
    },
    pull: async () => {},
    push: async () => {},
    cancelBusy: () => {
      cancelled = true;
    },
  });
  try {
    const fetch = labeled(window, "Fetch");
    const pull = labeled(window, "Pull");
    const push = labeled(window, "Push");
    expect(
      [...window.nodes.values()].some(
        (node) =>
          node.tag === NativeNodeTag.SwiftUIHost || node.tag === NativeNodeTag.SwiftUIButton,
      ),
    ).toBe(false);
    window._dispatchEvent("click", fetch.id, "");
    expect(fetches).toBe(1);
    setBusy({ label: "Fetch", cancel: () => {} });
    flush();
    expect(labeled(window, "Fetch")).toBe(fetch);
    expect(labeled(window, "Pull")).toBe(pull);
    expect(labeled(window, "Push")).toBe(push);
    expect(fetch.properties.get(PropertyCode.Disabled)).toBe(true);
    expect(labeled(window, "Git operation in progress").properties.get(PropertyCode.Part)).toBe(
      "progress",
    );
    const cancel = [...window.nodes.values()].find(
      (node) =>
        node.tag === NativeNodeTag.Button && node.children.some((child) => child.text === "Cancel"),
    )!;
    window._dispatchEvent("click", cancel.id, "");
    expect(cancelled).toBe(true);
    setBusy(undefined);
    setStatus({
      detached: true,
      headSha: "abcdef0123",
      hasUpstreamCounts: false,
      ahead: 0,
      behind: 0,
      stashCount: 0,
      entries: [],
    });
    flush();
    expect(labeled(window, "Fetch")).toBe(fetch);
    expect(fetch.properties.get(PropertyCode.Disabled)).toBe(false);
    expect(pull.properties.get(PropertyCode.Disabled)).toBe(true);
    expect(push.properties.get(PropertyCode.Disabled)).toBe(true);
  } finally {
    window.close();
  }
});

test("native splitter reports and persists widths without rebuilding pane contents", () => {
  const [width, setWidth] = createSignal(420);
  const window = mount(() =>
    createComponent(ResizablePanel, {
      label: "Resize history",
      get width() {
        return width();
      },
      onResize: setWidth,
      minimum: 260,
      maximum: 1000,
      get children() {
        return createComponent(Text, { children: "Retained selection" });
      },
    }),
  );
  try {
    const divider = labeled(window, "Resize history");
    const splitter = divider.parent!;
    const pane = splitter.children[0]!;
    const child = pane.children[0];
    expect(splitter.properties.get(PropertyCode.Part)).toBe("splitter");
    expect(splitter.properties.get(PropertyCode.Values)).toBe("[420,580]");
    expect(divider.properties.get(PropertyCode.HitSlopLeft)).toBe(4);
    expect(divider.listeners.has("pointer")).toBe(false);
    window._dispatchEvent("componentchange", splitter.id, JSON.stringify({ sizes: [460, 540] }));
    flush();
    expect(width()).toBe(460);
    expect(splitter.properties.get(PropertyCode.Values)).toBe("[460,540]");
    expect(splitter.children[0]).toBe(pane);
    expect(splitter.children[1]).toBe(divider);
    expect(pane.children[0]).toBe(child);
  } finally {
    window.close();
  }
});

test("sidebar selection follows the active view while keeping both rows mounted", () => {
  const [selected, setSelected] = createSignal("Changes");
  const window = mount(() =>
    createComponent(View, {
      get children() {
        return ["Changes", "History"].map((label) =>
          createComponent(NavRow, {
            label,
            get selected() {
              return selected() === label;
            },
            onClick: () => setSelected(label),
          }),
        );
      },
    }),
  );
  try {
    const changes = labeled(window, "Changes");
    const history = labeled(window, "History");
    window._dispatchEvent("click", history.id, "");
    flush();
    expect(labeled(window, "Changes")).toBe(changes);
    expect(labeled(window, "History")).toBe(history);
    expect(selected()).toBe("History");
    expect(history.properties.get(PropertyCode.BackgroundColor)).toBe(
      parseColor(themeFor("light").selection),
    );
    expect(changes.properties.get(PropertyCode.BackgroundColor)).toBe(parseColor("transparent"));
  } finally {
    window.close();
  }
});
