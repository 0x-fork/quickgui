import type { NativeNode } from "@quickgui/native";
import { For, Show, Text, View, createSignal, type Style } from "@quickgui/ui";
import { NavigationMenu } from "@quickgui/ui/base-ui";
import { ContextMenu, Menu, Menubar, PopoverMenu, useMenuItemState, type MenuItemDeclaration } from "@quickgui/ui/menu";
import { Popover, usePopoverPlacement } from "@quickgui/ui/popover";
import { Toast, useToastManager, type ToastDeclaration } from "@quickgui/ui/toast";
import { PreviewCard, Tooltip } from "@quickgui/ui/tooltip";
import type { ComponentItem } from "@quickgui/ui/types";

import { controlStyle, menuAppearance, menuRowStyle, p, popupStyle } from "../theme.ts";
import { Btn, Muted, Note, Panel, Row } from "../ui.tsx";

/* -------------------------------------------------------------------------------------------- *
 * Context menu
 * -------------------------------------------------------------------------------------------- */

const clipboardItems: MenuItemDeclaration[] = [
  { id: "cut", label: "Cut", shortcut: "⌘X" },
  { id: "copy", label: "Copy", shortcut: "⌘C" },
  { type: "separator" },
  { id: "paste", label: "Paste", shortcut: "⌘V", disabled: true },
];

function contextTarget(): Style {
  return {
    height: 64,
    borderRadius: 10,
    borderWidth: 1,
    borderColor: p().border,
    borderStyle: "dashed",
    alignItems: "center",
    justifyContent: "center",
    backgroundColor: p().panelAlt,
  };
}

export function ContextMenuDemo(): NativeNode {
  const [command, setCommand] = createSignal("nothing yet");
  const [parts, setParts] = createSignal("nothing yet");
  return (
    <Panel
      title="Context menu"
      hint="Two ways to answer a secondary click: the core's in-window surface from a declared JSON row model, and the same surface from Base UI row parts."
    >
      <ContextMenu.Root items={() => clipboardItems} appearance={menuAppearance()} onSelect={(details) => setCommand(details.id)}>
        <ContextMenu.Trigger style={contextTarget()}>
          <Muted text="Right-click: declared JSON rows" />
        </ContextMenu.Trigger>
      </ContextMenu.Root>

      {/* The child parts are the rows; the surface they sit on is still the declared appearance. */}
      <ContextMenu.Root appearance={menuAppearance()} onSelect={(details) => setParts(details.id)}>
        <ContextMenu.Trigger style={contextTarget()}>
          <Menu.Item value="rename" label="Rename" />
          <Menu.Item value="duplicate" label="Duplicate" />
          <Menu.Separator />
          <Menu.Item value="delete" label="Delete" />
          <Muted text="Right-click: the same Menu.Item parts" />
        </ContextMenu.Trigger>
      </ContextMenu.Root>

      <Note text={"json → " + command() + " · parts → " + parts()} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Menu
 * -------------------------------------------------------------------------------------------- */

interface MenuRowLabelProps {
  label: string;
}

function MenuRowLabel(props: MenuRowLabelProps): NativeNode {
  const state = useMenuItemState();
  return <Text style={{ fontSize: 12, color: state().highlighted ? p().accent : p().ink }}>{props.label}</Text>;
}

interface MenuRowProps {
  value: string;
  label: string;
  onActivate: (value: string) => void;
}

function MenuRow(props: MenuRowProps): NativeNode {
  return (
    <Menu.Item value={props.value} label={props.label} onClick={() => props.onActivate(props.value)} style={menuRowStyle()}>
      <MenuRowLabel label={props.label} />
    </Menu.Item>
  );
}

function MenuCheckboxIndicator(): NativeNode {
  const state = useMenuItemState();
  return (
    <Show when={() => state().checked === true}>
      <Menu.CheckboxItemIndicator>
        <Text style={{ fontSize: 12, color: p().accent }}>✓</Text>
      </Menu.CheckboxItemIndicator>
    </Show>
  );
}

function MenuRadioIndicator(): NativeNode {
  const state = useMenuItemState();
  return (
    <Show when={() => state().checked === true}>
      <Menu.RadioItemIndicator>
        <Text style={{ fontSize: 12, color: p().accent }}>●</Text>
      </Menu.RadioItemIndicator>
    </Show>
  );
}

const densities = ["compact", "cozy"];

export function MenuDemo(): NativeNode {
  const [open, setOpen] = createSignal(false);
  const [wrap, setWrap] = createSignal(false);
  const [density, setDensity] = createSignal<string | undefined>("cozy");
  const [activated, setActivated] = createSignal("nothing yet");
  return (
    <Panel
      title="Menu"
      hint="The Base UI menu compound: application-styled rows whose identity, keyboard navigation, roving highlight, toggle policy, radio exclusivity, and closing are all the core's."
    >
      <Menu.Root open={open} onOpenChange={(next) => setOpen(next)} side="bottom" align="start" sideOffset={6}>
        <Menu.Trigger style={controlStyle()}>
          <Text style={{ fontSize: 12, color: p().ink }}>Edit ▾</Text>
        </Menu.Trigger>
        <Menu.Positioner>
          <Menu.Popup style={[popupStyle(), { width: 250, padding: 5, gap: 2 }]}>
            <Menu.GroupLabel value="clipboard" label="Clipboard" style={{ paddingLeft: 10, height: 20, justifyContent: "center" }}>
              <Text style={{ fontSize: 11, color: p().faint }}>Clipboard</Text>
            </Menu.GroupLabel>
            <MenuRow value="copy" label="Copy" onActivate={(value) => setActivated(value)} />
            <MenuRow value="cut" label="Cut" onActivate={(value) => setActivated(value)} />
            <Menu.LinkItem
              value="docs"
              label="Documentation"
              href="https://quickgui.dev"
              style={menuRowStyle()}
              onNavigate={(href) => setActivated("link " + href)}
            >
              <MenuRowLabel label="Documentation" />
            </Menu.LinkItem>
            <Menu.Separator style={{ height: 1, backgroundColor: p().border, marginTop: 4, marginBottom: 4 }} />
            <Menu.CheckboxItem value="wrap" label="Wrap lines" checked={wrap} onCheckedChange={(next) => setWrap(next)} style={menuRowStyle()}>
              <MenuRowLabel label="Wrap lines" />
              <MenuCheckboxIndicator />
            </Menu.CheckboxItem>
            <Menu.RadioGroup name="density" value={density} onValueChange={(next) => setDensity(next)} style={{ display: "flex", flexDirection: "column" }}>
              <For each={() => densities}>
                {(value) => (
                  <Menu.RadioItem value={value} label={value} style={menuRowStyle()}>
                    <MenuRowLabel label={value} />
                    <MenuRadioIndicator />
                  </Menu.RadioItem>
                )}
              </For>
            </Menu.RadioGroup>
            <Menu.SubmenuRoot closeDelay={100}>
              <Menu.SubmenuTrigger value="recent" label="Open recent" openOnHover style={menuRowStyle()}>
                <MenuRowLabel label="Open recent" />
                <Text style={{ fontSize: 12, color: p().muted }}>›</Text>
              </Menu.SubmenuTrigger>
              <Menu.Positioner side="right" align="start" sideOffset={4}>
                <Menu.Popup style={[popupStyle(), { width: 200, padding: 5 }]}>
                  <MenuRow value="notes" label="notes.md" onActivate={(value) => setActivated(value)} />
                  <MenuRow value="readme" label="README.md" onActivate={(value) => setActivated(value)} />
                </Menu.Popup>
              </Menu.Positioner>
            </Menu.SubmenuRoot>
          </Menu.Popup>
        </Menu.Positioner>
      </Menu.Root>
      <Note text={"open " + String(open()) + " · activated " + activated() + " · wrap " + String(wrap()) + " · density " + (density() ?? "—")} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Menubar
 * -------------------------------------------------------------------------------------------- */

interface MenubarMenu {
  label: string;
  items: MenuItemDeclaration[];
}

const fileItems: MenuItemDeclaration[] = [
  { id: "new", label: "New window", shortcut: "⌘N" },
  { id: "open", label: "Open…", shortcut: "⌘O" },
  { type: "separator" },
  { id: "close", label: "Close", shortcut: "⌘W" },
];
const editItems: MenuItemDeclaration[] = [
  { id: "undo", label: "Undo", shortcut: "⌘Z" },
  { id: "redo", label: "Redo", shortcut: "⇧⌘Z" },
  { type: "separator" },
  { id: "paste", label: "Paste", shortcut: "⌘V" },
];
const viewItems: MenuItemDeclaration[] = [
  { id: "zoom-in", label: "Zoom in", shortcut: "⌘+" },
  { id: "zoom-out", label: "Zoom out", shortcut: "⌘−" },
];
const menubarMenus: MenubarMenu[] = [
  { label: "File", items: fileItems },
  { label: "Edit", items: editItems },
  { label: "View", items: viewItems },
];

export function MenubarDemo(): NativeNode {
  const [openMenu, setOpenMenu] = createSignal<number | undefined>(undefined);
  const [active, setActive] = createSignal(0);
  const [command, setCommand] = createSignal("nothing yet");
  return (
    <Panel
      title="Menubar"
      hint="One roving Tab stop across the bar. Arrow keys keep switching menus while one is open; Escape closes without leaving the bar."
    >
      <Menubar.Root
        count={menubarMenus.length}
        open={openMenu}
        onOpenChange={(index) => setOpenMenu(index)}
        onActiveChange={(index) => setActive(index)}
        style={{
          display: "flex",
          flexDirection: "row",
          gap: 2,
          padding: 4,
          borderRadius: 10,
          backgroundColor: p().panelAlt,
          borderWidth: 1,
          borderColor: p().border,
        }}
      >
        <For each={() => menubarMenus}>
          {(menu, index) => (
            <PopoverMenu.Root
              items={() => menu.items}
              appearance={menuAppearance()}
              open={() => openMenu() === index()}
              onOpenChange={(next) => setOpenMenu(next ? index() : undefined)}
              onSelect={(details) => setCommand(details.id)}
            >
              <PopoverMenu.Trigger style={{ display: "flex" }}>
                <Menubar.Item
                  index={index()}
                  style={[controlStyle(), { height: 26, backgroundColor: openMenu() === index() ? p().selection : "transparent", borderColor: "transparent" }]}
                >
                  <Text style={{ fontSize: 12, color: p().ink }}>{menu.label}</Text>
                </Menubar.Item>
              </PopoverMenu.Trigger>
              <PopoverMenu.Popup style={{ backgroundColor: p().popup, borderRadius: 10, width: 220 }} />
            </PopoverMenu.Root>
          )}
        </For>
      </Menubar.Root>
      <Note text={"open " + (openMenu() === undefined ? "none" : String(openMenu())) + " · tab stop " + String(active()) + " · command " + command()} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Navigation menu
 * -------------------------------------------------------------------------------------------- */

const navigationItems: ComponentItem[] = [{ value: "products" }, { value: "solutions" }, { value: "support", disabled: true }];
const navigationNames = ["products", "solutions", "support"];

export function NavigationMenuDemo(): NativeNode {
  const [value, setValue] = createSignal<string | undefined>(undefined);
  const [direction, setDirection] = createSignal("none");
  return (
    <Panel
      title="Navigation menu"
      hint="A Navigation landmark with one Tab stop. Hovering opens after the exact delay and switches immediately once a panel is open; the activation direction is reported so the panel can slide the right way."
    >
      <NavigationMenu.Root
        delay={80}
        closeDelay={80}
        value={value}
        onValueChange={(next) => setValue(next)}
        onActivationDirectionChange={(next) => setDirection(next ?? "none")}
        items={navigationItems}
      >
        <NavigationMenu.List style={{ display: "flex", flexDirection: "row", gap: 6 }}>
          <For each={() => navigationNames}>
            {(item) => (
              <NavigationMenu.Item value={item}>
                <NavigationMenu.Trigger
                  style={[controlStyle(), { opacity: item === "support" ? 0.5 : 1, backgroundColor: value() === item ? p().selection : p().control }]}
                >
                  <Text style={{ fontSize: 12, color: p().ink }}>{item}</Text>
                </NavigationMenu.Trigger>
                <NavigationMenu.Positioner>
                  <NavigationMenu.Popup style={[popupStyle(), { width: 230 }]}>
                    <NavigationMenu.Viewport>
                      <NavigationMenu.Content style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                        <NavigationMenu.Link value={item + "-overview"} active>
                          <Text style={{ fontSize: 12, color: p().accent }}>{item + " overview"}</Text>
                        </NavigationMenu.Link>
                        <NavigationMenu.Link value={item + "-pricing"}>
                          <Text style={{ fontSize: 12, color: p().ink }}>{item + " pricing"}</Text>
                        </NavigationMenu.Link>
                      </NavigationMenu.Content>
                    </NavigationMenu.Viewport>
                  </NavigationMenu.Popup>
                </NavigationMenu.Positioner>
              </NavigationMenu.Item>
            )}
          </For>
        </NavigationMenu.List>
      </NavigationMenu.Root>
      <Note text={"open panel " + (value() ?? "none") + " · activation direction " + direction()} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Popover
 * -------------------------------------------------------------------------------------------- */

function ResolvedPlacement(): NativeNode {
  const placement = usePopoverPlacement();
  return <Note text={"resolved " + placement().side + "/" + placement().align + " · anchor hidden " + String(placement().anchorHidden)} />;
}

export function PopoverDemo(): NativeNode {
  const [open, setOpen] = createSignal(false);
  const [hover, setHover] = createSignal(false);
  return (
    <Panel title="Popover" hint="The declared side is only a preference; the panel prints the placement the retained tree really resolved to.">
      <Popover.Root open={open} onOpenChange={(next) => setOpen(next)} side="bottom" align="start" sideOffset={8} collisionPadding={12}>
        <Row>
          <Popover.Trigger style={[controlStyle(), { backgroundColor: p().accent, borderColor: p().accent, hover: { backgroundColor: p().accentHover } }]}>
            <Text style={{ fontSize: 12, color: p().onAccent }}>Account</Text>
          </Popover.Trigger>
          <ResolvedPlacement />
        </Row>
        <Popover.Positioner>
          <Popover.Popup style={[popupStyle(), { width: 240 }]}>
            <Popover.Arrow style={{ width: 10, height: 10, backgroundColor: p().popup }} />
            <Popover.Title>
              <Text style={{ fontSize: 13, fontWeight: 700, color: p().ink }}>Signed in</Text>
            </Popover.Title>
            <Popover.Viewport style={{ maxHeight: 120 }}>
              <Muted text="ada@example.com" />
            </Popover.Viewport>
            <Popover.Close style={controlStyle()}>
              <Text style={{ fontSize: 12, color: p().ink }}>Done</Text>
            </Popover.Close>
          </Popover.Popup>
        </Popover.Positioner>
      </Popover.Root>

      <Popover.Root openOnHover delay={200} closeDelay={120} side="right" align="center" sideOffset={8} onOpenChange={(next) => setHover(next)}>
        <Popover.Trigger style={controlStyle()}>
          <Text style={{ fontSize: 12, color: p().ink }}>Hover to open</Text>
        </Popover.Trigger>
        <Popover.Positioner>
          <Popover.Popup style={[popupStyle(), { width: 200 }]}>
            <Muted text="Opened on the core's exact hover deadline." />
          </Popover.Popup>
        </Popover.Positioner>
      </Popover.Root>
      <Note text={"click popover " + String(open()) + " · hover popover " + String(hover())} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Preview card
 * -------------------------------------------------------------------------------------------- */

export function PreviewCardDemo(): NativeNode {
  const [open, setOpen] = createSignal(false);
  return (
    <Panel
      title="Preview card"
      hint="The trigger projects the Link role. Resting the pointer opens on the core's exact delay; focus opens it immediately, because a keyboard user cannot rest a pointer."
    >
      <Row>
        <Muted text="Written by" />
        <PreviewCard.Root open={open} onOpenChange={(next) => setOpen(next)} placement="bottom-start" gap={8}>
          <PreviewCard.Trigger delay={350} closeDelay={200} style={{ padding: 2 }}>
            <Text style={{ fontSize: 12, color: p().accent, textDecoration: "underline" }}>@ada</Text>
          </PreviewCard.Trigger>
          <PreviewCard.Positioner>
            <PreviewCard.Popup style={[popupStyle(), { width: 240 }]}>
              <Text style={{ fontSize: 13, fontWeight: 700, color: p().ink }}>Ada Lovelace</Text>
              <Muted text="Wrote the first algorithm intended for a machine." />
            </PreviewCard.Popup>
          </PreviewCard.Positioner>
        </PreviewCard.Root>
      </Row>
      <Note text={"open " + String(open())} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Toast
 * -------------------------------------------------------------------------------------------- */

function tooltipPopup(): Style {
  return { paddingLeft: 9, paddingRight: 9, paddingTop: 5, paddingBottom: 5, borderRadius: 7, backgroundColor: p().ink };
}

export function ToastDemo(): NativeNode {
  return (
    <Toast.Provider timeout={5000} limit={3} pitch={10} swipeDirection="right">
      <ToastDemoBody />
    </Toast.Provider>
  );
}

function toastTitle(toasts: ToastDeclaration[], id: string, fallback: string): string {
  for (const toast of toasts) if (toast.id === id) return toast.title;
  return fallback;
}

function toastDescription(toasts: ToastDeclaration[], id: string): string {
  for (const toast of toasts) if (toast.id === id) return toast.description ?? "";
  return "";
}

function ToastDemoBody(): NativeNode {
  const toasts = useToastManager();
  let counter = 0;
  let lastId = "";
  const limited = (): number => {
    let count = 0;
    for (const entry of toasts.stack()) if (entry.limited) count += 1;
    return count;
  };
  return (
    <Panel
      title="Toast"
      hint="A bounded queue with exact one-shot auto-dismiss deadlines. Older toasts past the limit are flagged and styled back, never silenced, and every stack offset is the core's."
    >
      <Row>
        <Btn
          label="Info"
          onClick={() => {
            counter += 1;
            lastId = toasts.add({ title: "Build " + String(counter) + " queued", description: "Auto-dismisses in 5s", type: "info" });
          }}
        />
        <Btn
          label="Success"
          onClick={() => {
            counter += 1;
            toasts.add({ title: "Build " + String(counter) + " finished", type: "success" });
          }}
        />
        <Btn
          label="Error"
          onClick={() => {
            counter += 1;
            toasts.add({ title: "Build " + String(counter) + " failed", type: "error", duration: 8000 });
          }}
        />
        <Btn
          label="Update last"
          onClick={() => {
            if (lastId !== "") toasts.update(lastId, { title: "Updated in place" });
          }}
        />
        <Btn label="Clear" onClick={() => toasts.closeAll()} />
      </Row>

      <Toast.Viewport style={{ display: "flex", flexDirection: "column", gap: 8, minHeight: 40 }}>
        <For each={() => toasts.stack()} key={(entry) => entry.id}>
          {(entry) => (
            <Toast.Positioner toastId={entry.id}>
              <Toast.Root
                toastId={entry.id}
                style={[
                  popupStyle(),
                  {
                    padding: 10,
                    gap: 4,
                    opacity: entry.limited ? 0.55 : 1,
                    transform: "translateX(" + String(entry.swipeMovement) + "px)",
                    borderColor: entry.type === "error" ? p().danger : entry.type === "success" ? p().accent : p().border,
                  },
                ]}
              >
                <Toast.Content toastId={entry.id}>
                  <Row>
                    <Toast.Title toastId={entry.id}>
                      <Text style={{ fontSize: 12, fontWeight: 700, color: p().ink }}>{toastTitle(toasts.toasts(), entry.id, entry.type)}</Text>
                    </Toast.Title>
                    <Muted text={"#" + String(entry.index) + " · +" + String(entry.offset) + "px"} />
                    <Toast.Close toastId={entry.id}>
                      <Text style={{ fontSize: 13, color: p().muted }}>×</Text>
                    </Toast.Close>
                  </Row>
                  <Toast.Description toastId={entry.id}>
                    <Muted text={toastDescription(toasts.toasts(), entry.id)} />
                  </Toast.Description>
                </Toast.Content>
              </Toast.Root>
            </Toast.Positioner>
          )}
        </For>
      </Toast.Viewport>
      <Note text={"queued " + String(toasts.toasts().length) + " · stack " + String(toasts.stack().length) + " · limited " + String(limited())} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Tooltip
 * -------------------------------------------------------------------------------------------- */

export function TooltipDemo(): NativeNode {
  const [open, setOpen] = createSignal(false);
  const [placement, setPlacement] = createSignal("—");
  return (
    <Panel
      title="Tooltip"
      hint="A provider shares one warm group deadline across its triggers, so the second tooltip in a row opens without waiting again. Like a native help tag, the popup never takes the pointer, so reaching it closes the tooltip; hoverable opts into Base UI's hover-through."
    >
      <Tooltip.Provider delay={500} closeDelay={120} timeout={400}>
        <Row>
          <Tooltip.Root side="top" sideOffset={8} onOpenChange={(next) => setOpen(next)} onPlacementChange={(details) => setPlacement(details.side + "/" + details.align)}>
            <Tooltip.Trigger style={controlStyle()}>
              <Text style={{ fontSize: 12, color: p().ink }}>Hover me</Text>
            </Tooltip.Trigger>
            <Tooltip.Positioner>
              <Tooltip.Popup style={tooltipPopup()}>
                <Text style={{ fontSize: 11, color: p().panel }}>The core owns every deadline</Text>
                <Tooltip.Arrow style={{ width: 8, height: 8, backgroundColor: p().ink }} />
              </Tooltip.Popup>
            </Tooltip.Positioner>
          </Tooltip.Root>

          <Tooltip.Root side="bottom" sideOffset={8} trackCursorAxis="x">
            <Tooltip.Trigger style={controlStyle()}>
              <Text style={{ fontSize: 12, color: p().ink }}>Tracks the cursor</Text>
            </Tooltip.Trigger>
            <Tooltip.Positioner>
              <Tooltip.Popup style={tooltipPopup()}>
                <Text style={{ fontSize: 11, color: p().panel }}>trackCursorAxis="x"</Text>
              </Tooltip.Popup>
            </Tooltip.Positioner>
          </Tooltip.Root>
        </Row>
      </Tooltip.Provider>
      <Note text={"open " + String(open()) + " · resolved " + placement()} />
    </Panel>
  );
}
