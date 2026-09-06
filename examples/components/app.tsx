import { Window, app, type NativeNode } from "@quickgui/native";
import { For, Show, Text, View, createRenderer, createSignal } from "@quickgui/ui";
import { Tabs } from "@quickgui/ui/tabs";

import { AccordionDemo, AlertDialogDemo, AvatarDemo, ButtonDemo, CheckboxDemo, CheckboxGroupDemo, CollapsibleDemo, DialogDemo } from "./demos/basics.tsx";
import { MeterDemo, ProgressDemo, RadioDemo, SeparatorDemo, SliderDemo, SplitterDemo, SwitchDemo, TabsDemo, ToggleDemo, ToggleGroupDemo, ToolbarDemo } from "./demos/controls.tsx";
import { ScrollAreaDemo, TableDemo, TreeDemo } from "./demos/data.tsx";
import { CalendarDemo, DateFieldDemo, FieldDemo, FieldsetDemo, FormDemo, InputDemo, NumberFieldDemo, OtpFieldDemo, TimeFieldDemo } from "./demos/forms.tsx";
import { ContextMenuDemo, MenuDemo, MenubarDemo, NavigationMenuDemo, PopoverDemo, PreviewCardDemo, ToastDemo, TooltipDemo } from "./demos/overlays.tsx";
import { AutocompleteDemo, ComboboxDemo, SelectDemo } from "./demos/pickers.tsx";
import { appearance, lightPalette, p, setAppearance, setViewportHeight, setViewportWidth, viewportHeight, viewportWidth } from "./theme.ts";

await app.whenReady();

/* -------------------------------------------------------------------------------------------- *
 * The gallery shell
 * -------------------------------------------------------------------------------------------- */

interface Demo {
  id: string;
  label: string;
  /** Where the component's shape comes from: Base UI's catalog, or QuickGUI's own set. */
  source?: string;
  render: () => NativeNode;
}

const DEMOS: Demo[] = [
  { id: "accordion", label: "Accordion", render: () => <AccordionDemo /> },
  { id: "alert-dialog", label: "Alert Dialog", render: () => <AlertDialogDemo /> },
  { id: "autocomplete", label: "Autocomplete", render: () => <AutocompleteDemo /> },
  { id: "avatar", label: "Avatar", render: () => <AvatarDemo /> },
  { id: "button", label: "Button", render: () => <ButtonDemo /> },
  { id: "calendar", source: "QuickGUI", label: "Calendar", render: () => <CalendarDemo /> },
  { id: "checkbox", label: "Checkbox", render: () => <CheckboxDemo /> },
  { id: "checkbox-group", label: "Checkbox Group", render: () => <CheckboxGroupDemo /> },
  { id: "collapsible", label: "Collapsible", render: () => <CollapsibleDemo /> },
  { id: "combobox", label: "Combobox", render: () => <ComboboxDemo /> },
  { id: "context-menu", label: "Context Menu", render: () => <ContextMenuDemo /> },
  { id: "date-field", source: "QuickGUI", label: "Date Field", render: () => <DateFieldDemo /> },
  { id: "dialog", label: "Dialog", render: () => <DialogDemo /> },
  { id: "field", label: "Field", render: () => <FieldDemo /> },
  { id: "fieldset", label: "Fieldset", render: () => <FieldsetDemo /> },
  { id: "form", label: "Form", render: () => <FormDemo /> },
  { id: "input", label: "Input", render: () => <InputDemo /> },
  { id: "menu", label: "Menu", render: () => <MenuDemo /> },
  { id: "menubar", label: "Menubar", render: () => <MenubarDemo /> },
  { id: "meter", label: "Meter", render: () => <MeterDemo /> },
  { id: "navigation-menu", label: "Navigation Menu", render: () => <NavigationMenuDemo /> },
  { id: "number-field", label: "Number Field", render: () => <NumberFieldDemo /> },
  { id: "otp-field", label: "OTP Field", render: () => <OtpFieldDemo /> },
  { id: "popover", label: "Popover", render: () => <PopoverDemo /> },
  { id: "preview-card", label: "Preview Card", render: () => <PreviewCardDemo /> },
  { id: "progress", label: "Progress", render: () => <ProgressDemo /> },
  { id: "radio", label: "Radio", render: () => <RadioDemo /> },
  { id: "scroll-area", label: "Scroll Area", render: () => <ScrollAreaDemo /> },
  { id: "select", label: "Select", render: () => <SelectDemo /> },
  { id: "separator", label: "Separator", render: () => <SeparatorDemo /> },
  { id: "slider", label: "Slider", render: () => <SliderDemo /> },
  { id: "splitter", source: "QuickGUI", label: "Splitter", render: () => <SplitterDemo /> },
  { id: "switch", label: "Switch", render: () => <SwitchDemo /> },
  { id: "tabs", label: "Tabs", render: () => <TabsDemo /> },
  { id: "time-field", source: "QuickGUI", label: "Time Field", render: () => <TimeFieldDemo /> },
  { id: "toast", label: "Toast", render: () => <ToastDemo /> },
  { id: "toggle", label: "Toggle", render: () => <ToggleDemo /> },
  { id: "toggle-group", label: "Toggle Group", render: () => <ToggleGroupDemo /> },
  { id: "toolbar", label: "Toolbar", render: () => <ToolbarDemo /> },
  { id: "tooltip", label: "Tooltip", render: () => <TooltipDemo /> },
  { id: "table", source: "QuickGUI", label: "Table", render: () => <TableDemo /> },
  { id: "tree", source: "QuickGUI", label: "Tree", render: () => <TreeDemo /> },
];

/** The catalog a demo's parts and props follow. */
function sourceOf(demo: Demo | undefined): string {
  if (demo === undefined) return "";
  return demo.source === "QuickGUI" ? "QuickGUI component" : "Base UI part set";
}

function findDemo(id: string | undefined): Demo | undefined {
  for (const demo of DEMOS) if (demo.id === id) return demo;
  return undefined;
}

function Gallery(): NativeNode {
  const [tab, setTab] = createSignal<string | undefined>("accordion");
  const current = (): Demo | undefined => findDemo(tab());
  const currentLabel = (): string => {
    const demo = current();
    return demo === undefined ? "" : demo.label;
  };
  return (
    <View style={{ display: "flex", flexDirection: "row", width: "100%", height: "100%", backgroundColor: p().window }}>
      <Tabs.Root
        value={tab}
        onValueChange={(next) => setTab(next)}
        orientation="vertical"
        activation="manual"
        style={{ display: "flex", flexDirection: "row", width: "100%", height: "100%" }}
      >
        {/* Sidebar: one vertical tab per component. */}
        <View
          style={{
            display: "flex",
            flexDirection: "column",
            width: 214,
            flexShrink: 0,
            height: "100%",
            backgroundColor: p().sidebar,
            borderRightWidth: 1,
            borderColor: p().border,
          }}
        >
          {/* The traffic lights sit at (16, 18); the title clears them and centres on their row. */}
          <View style={{ display: "flex", flexDirection: "row", alignItems: "center", height: 52, flexShrink: 0, paddingLeft: 82, appRegion: "drag" }}>
            <Text style={{ fontSize: 13, fontWeight: 700, color: p().ink }}>Components</Text>
          </View>
          <View style={{ display: "flex", flexDirection: "column", flex: 1, minHeight: 0, overflowY: "scroll" }}>
            <Tabs.List style={{ display: "flex", flexDirection: "column", gap: 1, paddingLeft: 8, paddingRight: 8, paddingBottom: 12 }}>
              <For each={() => DEMOS}>
                {(demo, index) => (
                  <Tabs.Tab
                    value={demo.id}
                    index={index()}
                    style={{
                      display: "flex",
                      flexDirection: "row",
                      alignItems: "center",
                      height: 28,
                      paddingLeft: 10,
                      paddingRight: 10,
                      borderRadius: 7,
                      cursor: "default",
                      userSelect: "none",
                      backgroundColor: tab() === demo.id ? p().accent : "transparent",
                      hover: { backgroundColor: tab() === demo.id ? p().accent : p().controlHover },
                      focus: { outline: "2px solid " + p().accent },
                      outlineOffset: -2,
                    }}
                  >
                    <Text style={{ fontSize: 12, fontWeight: tab() === demo.id ? 600 : 400, color: tab() === demo.id ? p().onAccent : p().ink }}>{demo.label}</Text>
                  </Tabs.Tab>
                )}
              </For>
            </Tabs.List>
          </View>
        </View>

        {/* Content: the selected demo, in a scrollable pane. */}
        <View style={{ display: "flex", flexDirection: "column", flex: 1, height: "100%" }}>
          <View
            style={{
              height: 52,
              flexShrink: 0,
              display: "flex",
              flexDirection: "row",
              alignItems: "center",
              justifyContent: "space-between",
              paddingLeft: 20,
              paddingRight: 20,
              borderBottomWidth: 1,
              borderColor: p().border,
              appRegion: "drag",
            }}
          >
            <View style={{ display: "flex", flexDirection: "row", alignItems: "baseline", gap: 10 }}>
              <Text style={{ fontSize: 15, fontWeight: 700, color: p().ink }}>{currentLabel()}</Text>
              <Text style={{ fontSize: 11, color: p().faint }}>{sourceOf(current())}</Text>
            </View>
            <Text style={{ fontSize: 11, color: p().faint }}>
              {String(DEMOS.length) + " components · " + appearance() + " appearance · " + String(Math.round(viewportWidth())) + "×" + String(Math.round(viewportHeight()))}
            </Text>
          </View>
          <View style={{ flex: 1, overflowY: "scroll", display: "flex", flexDirection: "column", gap: 16, padding: 20 }}>
            <For each={() => DEMOS}>
              {(demo) => (
                <Show when={() => tab() === demo.id}>
                  <Tabs.Panel value={demo.id} style={{ display: "flex", flexDirection: "column", gap: 16, maxWidth: 720 }}>
                    {demo.render()}
                  </Tabs.Panel>
                </Show>
              )}
            </For>
          </View>
        </View>
      </Tabs.Root>
    </View>
  );
}

function openMainWindow(): Window {
  const window = new Window({
    title: "QuickGUI Components",
    width: 1080,
    height: 780,
    minimumWidth: 880,
    minimumHeight: 620,
    background: lightPalette.window,
    titleBarStyle: "hiddenInset",
    trafficLightPosition: { x: 16, y: 18 },
    renderer: createRenderer(() => <Gallery />),
  });
  // The appearance is read from the window the core really opened, and every later change is
  // applied from the window's own events, never from a component body.
  window.on("appearanceChange", (event) => {
    if (event.appearance !== undefined) setAppearance(event.appearance);
  });
  window.on("resize", (event) => {
    const size = event.size;
    if (size === undefined) return;
    setViewportWidth(size.width);
    setViewportHeight(size.height);
  });
  // The native window mounts on the next host turn, so its first state is read once it is ready.
  window.on("readyToShow", () => {
    void adoptWindowState(window);
  });
  return window;
}

async function adoptWindowState(window: Window): Promise<void> {
  try {
    const state = await window.getState();
    setAppearance(state.appearance);
    setViewportWidth(state.viewportSize.width);
    setViewportHeight(state.viewportSize.height);
  } catch {
    // The window closed before its first state could be read.
  }
}

openMainWindow();

app.onReopen((event) => {
  if (!event.hasVisibleWindows) openMainWindow();
});
