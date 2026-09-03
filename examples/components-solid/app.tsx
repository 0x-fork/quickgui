import { app, Window } from "@quickgui/native";
import {
  Autocomplete,
  Button,
  Calendar,
  Combobox,
  ContextMenu,
  DateField,
  Dialog,
  Menubar,
  NumberField,
  PopoverMenu,
  Select,
  Slider,
  Splitter,
  Table,
  Tabs,
  Text,
  TimeField,
  Toast,
  ToggleGroup,
  Toolbar,
  Tree,
  View,
  type ToastDeclaration,
  type TreeNodeDeclaration,
  type VisibleRange,
} from "@quickgui/solid";
import { createRenderer } from "@quickgui/solid";
import { createMemo, createSignal, For } from "solid-js";

await app.whenReady();

const ink = "#e6ecf7";
const muted = "#94a3b8";
const surface = "#141a26";
const border = "#27324a";
const accent = "#2563eb";

const panelStyle = {
  display: "flex",
  flexDirection: "column",
  gap: 12,
  padding: 16,
  backgroundColor: surface,
  borderColor: border,
  borderWidth: 1,
  borderRadius: 12,
} as const;

const controlStyle = {
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  height: 30,
  paddingLeft: 12,
  paddingRight: 12,
  borderRadius: 8,
  backgroundColor: "#1b2434",
  borderColor: border,
  borderWidth: 1,
  color: ink,
  fontSize: 13,
  cursor: "default",
  userSelect: "none",
  hoverBackgroundColor: "#243044",
  focusOutline: "2px solid #60a5fa",
  outlineOffset: 2,
} as const;

const captionStyle = {
  color: muted,
  fontSize: 11,
  letterSpacing: 0.8,
  textTransform: "uppercase",
  fontWeight: 700,
} as const;

function Panel(props: { title: string; children?: unknown }) {
  return (
    <View style={panelStyle}>
      <Text style={captionStyle}>{props.title}</Text>
      {props.children as never}
    </View>
  );
}

function Row(props: { children?: unknown }) {
  return (
    <View
      style={{
        display: "flex",
        flexDirection: "row",
        alignItems: "center",
        gap: 10,
        flexWrap: "wrap",
      }}
    >
      {props.children as never}
    </View>
  );
}

/** Select, Combobox, and Autocomplete each open a native popover the core renders itself. */
function OptionSources() {
  const [theme, setTheme] = createSignal("dark");
  const [fruit, setFruit] = createSignal("apple");
  const [query, setQuery] = createSignal("");
  const appearance = {
    width: 240,
    rowHeight: 28,
    background: "#101725",
    color: ink,
    highlightBackground: accent,
    highlightColor: "#ffffff",
    mutedColor: muted,
  } as const;

  return (
    <Panel title="Option sources">
      <Row>
        <Select.Root
          scope="theme"
          ariaLabel="Theme"
          value={theme()}
          items={[
            { value: "light", label: "Light" },
            { value: "dark", label: "Dark", detail: "⌘D" },
            { value: "system", label: "Match system" },
          ]}
          appearance={appearance}
          onValueChange={(next) => setTheme(next ?? "")}
          style={{ ...controlStyle, width: 180 }}
        >
          <Text style={{ fontSize: 13, color: ink }}>{theme()}</Text>
        </Select.Root>

        <Combobox.Root
          scope="fruit"
          ariaLabel="Fruit"
          placeholder="Constrained combobox"
          value={fruit()}
          appearance={appearance}
          onValueChange={(next) => setFruit(next ?? "")}
          style={{ ...controlStyle, width: 200, justifyContent: "flex-start" }}
        >
          <Combobox.Option scope="fruit" partValue="apple" label="Apple" group="Common" />
          <Combobox.Option scope="fruit" partValue="banana" label="Banana" group="Common" />
          <Combobox.Option scope="fruit" partValue="lychee" label="Lychee" group="Tropical" />
        </Combobox.Root>

        <Autocomplete.Root
          scope="search"
          ariaLabel="Search"
          placeholder="Free-form autocomplete"
          inputValue={query()}
          items={[
            { value: "window", label: "Window" },
            { value: "webview", label: "Web view" },
            { value: "widget", label: "Widget" },
          ]}
          appearance={appearance}
          onInputValueChange={setQuery}
          style={{ ...controlStyle, width: 220, justifyContent: "flex-start" }}
        />
      </Row>
      <Text style={{ fontSize: 12, color: muted }}>
        {`theme ${theme()} · fruit ${fruit()} · query "${query()}"`}
      </Text>
    </Panel>
  );
}

/** A virtual table declares only the rows the core last reported as visible. */
function VirtualTable() {
  const rows = Array.from({ length: 5_000 }, (_, index) => ({
    name: `asset-${String(index).padStart(4, "0")}.png`,
    size: `${(index % 900) + 12} KB`,
  }));
  const [range, setRange] = createSignal<VisibleRange>({ start: 0, end: 0 });
  const [selection, setSelection] = createSignal<readonly (readonly number[])[]>([]);
  const visible = createMemo(() => rows.slice(range().start, range().end));

  return (
    <Panel title="Virtual table">
      <Table.Root
        scope="files"
        rowCount={rows.length}
        rowHeight={26}
        headerHeight={28}
        selectionMode="multiple"
        selection={selection()}
        columns={[
          { id: "name", label: "Name", width: 220, sortable: true, rowHeader: true },
          { id: "size", label: "Size", track: "1fr", align: "end" },
        ]}
        onVisibleRangeChange={setRange}
        onSelectionChange={setSelection}
        style={{
          height: 200,
          borderRadius: 10,
          borderWidth: 1,
          borderColor: border,
          overflowY: "scroll",
        }}
      >
        <Table.Header column="name" style={{ paddingLeft: 8, justifyContent: "center" }}>
          <Text style={{ fontSize: 12, fontWeight: 700, color: muted }}>Name</Text>
        </Table.Header>
        <Table.Header column="size" style={{ paddingRight: 8, justifyContent: "center" }}>
          <Text style={{ fontSize: 12, fontWeight: 700, color: muted }}>Size</Text>
        </Table.Header>
        <For each={visible()}>
          {(row, index) => (
            <Table.Row index={range().start + index()}>
              <Table.Cell column="name" style={{ paddingLeft: 8, justifyContent: "center" }}>
                <Text style={{ fontSize: 12, color: ink }}>{row.name}</Text>
              </Table.Cell>
              <Table.Cell column="size" style={{ paddingRight: 8, justifyContent: "center" }}>
                <Text style={{ fontSize: 12, color: muted, textAlign: "end" }}>{row.size}</Text>
              </Table.Cell>
            </Table.Row>
          )}
        </For>
      </Table.Root>
      <Text style={{ fontSize: 12, color: muted }}>
        {`rows ${range().start}–${range().end} of ${rows.length} · ${selection().length} selected range(s)`}
      </Text>
    </Panel>
  );
}

/** A tree whose one pending branch asks for its children exactly once. */
function LazyTree() {
  const nodes: readonly TreeNodeDeclaration[] = [
    {
      id: "src",
      label: "src",
      children: [
        { id: "src/lib.rs", label: "lib.rs" },
        { id: "src/element", label: "element", pending: true },
      ],
    },
    { id: "docs", label: "docs", children: [{ id: "docs/solid.md", label: "solid.md" }] },
  ];
  const [expanded, setExpanded] = createSignal<readonly string[]>(["src"]);
  const [loaded, setLoaded] = createSignal<
    { id: string; children: readonly TreeNodeDeclaration[] } | undefined
  >();
  // `setChildren` is declared only after the core asks for the branch, so the prop is absent
  // rather than explicitly undefined until then.
  const lazyChildren = () => {
    const splice = loaded();
    return splice ? { setChildren: splice } : {};
  };
  const [range, setRange] = createSignal<VisibleRange>({ start: 0, end: 0 });

  const flattened = createMemo(() => {
    const splice = loaded();
    const walk = (list: readonly TreeNodeDeclaration[]): TreeNodeDeclaration[] =>
      list.flatMap((node) => {
        const children =
          splice && splice.id === node.id ? splice.children : (node.children ?? []);
        return expanded().includes(node.id) ? [node, ...walk(children)] : [node];
      });
    return walk(nodes);
  });
  const visible = createMemo(() => flattened().slice(range().start, range().end));

  return (
    <Panel title="Lazy tree">
      <Tree.Root
        scope="explorer"
        nodes={nodes}
        rowHeight={26}
        loadingLabel="Loading…"
        expanded={expanded()}
        {...lazyChildren()}
        onExpandedChange={setExpanded}
        onVisibleRangeChange={setRange}
        onLoadChildren={(id) =>
          setLoaded({
            id,
            children: [
              { id: `${id}/style.rs`, label: "style.rs" },
              { id: `${id}/state.rs`, label: "state.rs" },
            ],
          })
        }
        style={{
          height: 180,
          borderRadius: 10,
          borderWidth: 1,
          borderColor: border,
          overflowY: "scroll",
        }}
      >
        <For each={visible()}>
          {(node) => (
            <Tree.Row nodeId={node.id} style={{ paddingLeft: 8, justifyContent: "center" }}>
              <Text style={{ fontSize: 12, color: ink }}>{node.label ?? node.id}</Text>
            </Tree.Row>
          )}
        </For>
      </Tree.Root>
    </Panel>
  );
}

/** Slider, NumberField, Splitter, Toolbar, and ToggleGroup all name one retained instance. */
function RangeAndOrdering() {
  const [volume, setVolume] = createSignal<readonly number[]>([40]);
  const [quantity, setQuantity] = createSignal(3);
  const [sizes, setSizes] = createSignal<readonly number[]>([180, 180]);
  const [tool, setTool] = createSignal("select");
  const [formats, setFormats] = createSignal<readonly string[]>(["bold"]);

  return (
    <Panel title="Range, ordering, and roving focus">
      <Slider.Root
        scope="volume"
        value={volume()}
        min={0}
        max={100}
        step={5}
        largeStep={25}
        onValueChange={setVolume}
        style={{ height: 24, justifyContent: "center" }}
      >
        <Slider.Track
          scope="volume"
          style={{ height: 4, borderRadius: 2, backgroundColor: "#243044" }}
        >
          <Slider.Range
            scope="volume"
            style={{ height: 4, borderRadius: 2, backgroundColor: accent }}
          />
        </Slider.Track>
        <Slider.Thumb
          scope="volume"
          itemIndex={0}
          style={{
            position: "absolute",
            width: 14,
            height: 14,
            borderRadius: 7,
            backgroundColor: "#f8fafc",
            focusOutline: "2px solid #60a5fa",
            outlineOffset: 2,
          }}
        />
      </Slider.Root>

      <Row>
        <NumberField.Root
          scope="qty"
          value={quantity()}
          min={0}
          max={99}
          step={1}
          precision={0}
          onValueChange={(next) => setQuantity(next ?? 0)}
          style={{ display: "flex", flexDirection: "row", gap: 4, alignItems: "center" }}
        >
          <NumberField.Decrement scope="qty" style={{ ...controlStyle, width: 30 }}>
            <Text style={{ fontSize: 14, color: ink }}>−</Text>
          </NumberField.Decrement>
          <NumberField.Input
            scope="qty"
            style={{ ...controlStyle, width: 72, textAlign: "center" }}
          />
          <NumberField.Increment scope="qty" style={{ ...controlStyle, width: 30 }}>
            <Text style={{ fontSize: 14, color: ink }}>+</Text>
          </NumberField.Increment>
        </NumberField.Root>

        <Toolbar.Root
          scope="tools"
          items={[{ value: "select" }, { value: "pen" }, { value: "erase", disabled: true }]}
          active={tool()}
          onActiveChange={(next) => setTool(next ?? "select")}
          style={{ display: "flex", flexDirection: "row", gap: 4 }}
        >
          <Toolbar.Item scope="tools" partValue="select" style={controlStyle}>
            <Text style={{ fontSize: 12, color: ink }}>Select</Text>
          </Toolbar.Item>
          <Toolbar.Item scope="tools" partValue="pen" style={controlStyle}>
            <Text style={{ fontSize: 12, color: ink }}>Pen</Text>
          </Toolbar.Item>
          <Toolbar.Item scope="tools" partValue="erase" style={controlStyle}>
            <Text style={{ fontSize: 12, color: muted }}>Erase</Text>
          </Toolbar.Item>
        </Toolbar.Root>

        <ToggleGroup.Root
          scope="format"
          variant="multiple"
          items={[{ value: "bold" }, { value: "italic" }, { value: "underline" }]}
          value={formats()}
          onValueChange={setFormats}
          style={{ display: "flex", flexDirection: "row", gap: 4 }}
        >
          <ToggleGroup.Item scope="format" partValue="bold" style={controlStyle}>
            <Text style={{ fontSize: 12, fontWeight: 700, color: ink }}>B</Text>
          </ToggleGroup.Item>
          <ToggleGroup.Item scope="format" partValue="italic" style={controlStyle}>
            <Text style={{ fontSize: 12, color: ink }}>I</Text>
          </ToggleGroup.Item>
          <ToggleGroup.Item scope="format" partValue="underline" style={controlStyle}>
            <Text style={{ fontSize: 12, textDecoration: "underline", color: ink }}>U</Text>
          </ToggleGroup.Item>
        </ToggleGroup.Root>
      </Row>

      <Splitter.Root
        scope="panes"
        value={sizes()}
        panes={[{ min: 80 }, { min: 80, collapsible: true }]}
        onSizesChange={setSizes}
        style={{
          display: "flex",
          flexDirection: "row",
          height: 90,
          borderRadius: 10,
          borderWidth: 1,
          borderColor: border,
        }}
      >
        <Splitter.Pane
          scope="panes"
          itemIndex={0}
          style={{ alignItems: "center", justifyContent: "center" }}
        >
          <Text style={{ fontSize: 12, color: muted }}>{`pane 0 · ${Math.round(sizes()[0] ?? 0)}`}</Text>
        </Splitter.Pane>
        <Splitter.Handle
          scope="panes"
          itemIndex={0}
          style={{ width: 6, backgroundColor: border, cursor: "col-resize" }}
        />
        <Splitter.Pane
          scope="panes"
          itemIndex={1}
          style={{ alignItems: "center", justifyContent: "center" }}
        >
          <Text style={{ fontSize: 12, color: muted }}>{`pane 1 · ${Math.round(sizes()[1] ?? 0)}`}</Text>
        </Splitter.Pane>
      </Splitter.Root>

      <Text style={{ fontSize: 12, color: muted }}>
        {`volume ${volume()[0] ?? 0} · quantity ${quantity()} · tool ${tool()} · ${formats().join(", ") || "no format"}`}
      </Text>
    </Panel>
  );
}

/** Civil date and time values cross the boundary as ISO strings with no time zone. */
function DateAndTime() {
  const [due, setDue] = createSignal("2026-09-03");
  const [at, setAt] = createSignal("09:30");
  const [day, setDay] = createSignal("2026-09-03");
  const [month, setMonth] = createSignal("2026-09");

  const grid = createMemo(() => {
    const [year, monthNumber] = month().split("-").map(Number);
    const first = new Date(Date.UTC(year ?? 2026, (monthNumber ?? 1) - 1, 1));
    const start = first.getUTCDay() === 0 ? 6 : first.getUTCDay() - 1;
    return Array.from({ length: 6 }, (_, week) =>
      Array.from({ length: 7 }, (_, weekday) => {
        const date = new Date(first);
        date.setUTCDate(1 - start + week * 7 + weekday);
        return date.toISOString().slice(0, 10);
      }),
    );
  });

  const segmentStyle = { ...controlStyle, width: 44, height: 26 } as const;

  return (
    <Panel title="Dates and times">
      <Row>
        <DateField.Root
          scope="due"
          value={due()}
          format="mdy"
          onValueChange={(next) => setDue(next ?? "")}
          style={{ display: "flex", flexDirection: "row", gap: 4 }}
        >
          <DateField.Segment scope="due" segment="month" style={segmentStyle} />
          <DateField.Segment scope="due" segment="day" style={segmentStyle} />
          <DateField.Segment scope="due" segment="year" style={{ ...segmentStyle, width: 64 }} />
        </DateField.Root>

        <TimeField.Root
          scope="at"
          value={at()}
          hour12
          onValueChange={(next) => setAt(next ?? "")}
          style={{ display: "flex", flexDirection: "row", gap: 4 }}
        >
          <TimeField.Segment scope="at" segment="hour" style={segmentStyle} />
          <TimeField.Segment scope="at" segment="minute" style={segmentStyle} />
          <TimeField.Segment scope="at" segment="period" style={segmentStyle} />
        </TimeField.Root>
      </Row>

      <Calendar.Root
        scope="calendar"
        value={day()}
        firstWeekday={0}
        onValueChange={(next) => setDay(next ?? "")}
        onMonthChange={setMonth}
        style={{ display: "flex", flexDirection: "column", gap: 2 }}
      >
        <For each={grid()}>
          {(week, index) => (
            <Calendar.Week
              scope="calendar"
              itemIndex={index()}
              style={{ display: "flex", flexDirection: "row", gap: 2 }}
            >
              <For each={week}>
                {(date) => (
                  <Calendar.Day
                    scope="calendar"
                    day={date}
                    style={{
                      width: 30,
                      height: 26,
                      borderRadius: 6,
                      alignItems: "center",
                      justifyContent: "center",
                      backgroundColor: date === day() ? accent : "#1b2434",
                      hoverBackgroundColor: "#243044",
                      focusOutline: "2px solid #60a5fa",
                    }}
                  >
                    <Text style={{ fontSize: 11, color: ink }}>{date.slice(8)}</Text>
                  </Calendar.Day>
                )}
              </For>
            </Calendar.Week>
          )}
        </For>
      </Calendar.Root>

      <Text style={{ fontSize: 12, color: muted }}>
        {`due ${due()} · at ${at()} · selected ${day()} · showing ${month()}`}
      </Text>
    </Panel>
  );
}

/** Menubar, popover menus, and the cursor-point context menu the core renders natively. */
function Menus() {
  const [openMenu, setOpenMenu] = createSignal<number | null>(null);
  const [command, setCommand] = createSignal("no command yet");
  const menus = [
    {
      label: "File",
      items: [
        { id: "new", label: "New window", shortcut: "⌘N" },
        { id: "open", label: "Open…", shortcut: "⌘O" },
      ],
    },
    {
      label: "Edit",
      items: [
        { id: "undo", label: "Undo", shortcut: "⌘Z" },
        { type: "separator" as const },
        { id: "paste", label: "Paste", shortcut: "⌘V" },
      ],
    },
  ];
  const appearance = {
    width: 220,
    background: "#101725",
    color: ink,
    highlightBackground: accent,
    mutedColor: muted,
  } as const;

  return (
    <Panel title="Menus">
      <Menubar.Root
        scope="bar"
        count={menus.length}
        open={openMenu()}
        onOpenChange={(index) => setOpenMenu(index ?? null)}
        style={{ display: "flex", flexDirection: "row", gap: 4 }}
      >
        <For each={menus}>
          {(menu, index) => (
            <PopoverMenu.Root
              items={menu.items}
              appearance={appearance}
              open={openMenu() === index()}
              onOpenChange={(open) => setOpenMenu(open ? index() : null)}
              onSelect={(details) => setCommand(`menubar → ${details.id}`)}
            >
              <PopoverMenu.Trigger style={{ display: "flex" }}>
                <Menubar.Item scope="bar" itemIndex={index()} style={controlStyle}>
                  <Text style={{ fontSize: 12, color: ink }}>{menu.label}</Text>
                </Menubar.Item>
              </PopoverMenu.Trigger>
              <PopoverMenu.Popup
                style={{ backgroundColor: "#101725", borderRadius: 8, width: 220 }}
              />
            </PopoverMenu.Root>
          )}
        </For>
      </Menubar.Root>

      <ContextMenu.Root
        items={[
          { id: "cut", label: "Cut", shortcut: "⌘X" },
          { id: "copy", label: "Copy", shortcut: "⌘C" },
          { type: "separator" },
          { id: "paste", label: "Paste", shortcut: "⌘V" },
        ]}
        appearance={appearance}
        onSelect={(details) => setCommand(`context → ${details.id}`)}
      >
        <ContextMenu.Trigger
          style={{
            height: 60,
            borderRadius: 10,
            borderWidth: 1,
            borderColor: border,
            borderStyle: "dashed",
            alignItems: "center",
            justifyContent: "center",
          }}
        >
          <Text style={{ fontSize: 12, color: muted }}>Right-click for a native menu</Text>
        </ContextMenu.Trigger>
      </ContextMenu.Root>

      <Text style={{ fontSize: 12, color: muted }}>{command()}</Text>
    </Panel>
  );
}

/** Tabs, an in-window dialog, and the core-owned toast queue. */
function SurfacesAndFeedback() {
  const [tab, setTab] = createSignal("overview");
  const [dialogOpen, setDialogOpen] = createSignal(false);
  const [toasts, setToasts] = createSignal<readonly ToastDeclaration[]>([]);
  let nextToast = 0;

  return (
    <Panel title="Tabs, dialogs, and toasts">
      <Tabs.Root value={tab()} onValueChange={setTab} activation="automatic">
        <Tabs.List style={{ display: "flex", flexDirection: "row", gap: 4 }}>
          <Tabs.Tab value="overview" style={controlStyle}>
            <Text style={{ fontSize: 12, color: ink }}>Overview</Text>
            <Tabs.Indicator
              style={{
                position: "absolute",
                bottom: 0,
                left: 0,
                right: 0,
                height: 2,
                backgroundColor: accent,
              }}
            />
          </Tabs.Tab>
          <Tabs.Tab value="usage" style={controlStyle}>
            <Text style={{ fontSize: 12, color: ink }}>Usage</Text>
          </Tabs.Tab>
        </Tabs.List>
        <Tabs.Panel value="overview" style={{ paddingTop: 8 }}>
          <Text style={{ fontSize: 12, color: muted }}>
            An inactive panel is not mounted at all, so it contributes no layout or paint.
          </Text>
        </Tabs.Panel>
        <Tabs.Panel value="usage" style={{ paddingTop: 8 }}>
          <Text style={{ fontSize: 12, color: muted }}>
            Arrow keys move the tab stop; `activation="automatic"` selects as focus moves.
          </Text>
        </Tabs.Panel>
      </Tabs.Root>

      <Row>
        <Dialog.Root open={dialogOpen()} onOpenChange={setDialogOpen}>
          <Dialog.Trigger style={controlStyle}>
            <Text style={{ fontSize: 12, color: ink }}>Open dialog</Text>
          </Dialog.Trigger>
          <Dialog.Portal>
            <Dialog.Backdrop
              style={{ position: "absolute", top: 0, right: 0, bottom: 0, left: 0, backgroundColor: "#0f172acc" }}
            />
            <Dialog.Popup
              style={{
                width: 320,
                gap: 10,
                padding: 20,
                borderRadius: 14,
                backgroundColor: surface,
                borderColor: border,
                borderWidth: 1,
                display: "flex",
                flexDirection: "column",
              }}
            >
              <Dialog.Title style={{ fontSize: 15, fontWeight: 700, color: ink }}>
                Publish this build?
              </Dialog.Title>
              <Dialog.Description style={{ fontSize: 12, color: muted }}>
                The core owns the focus trap and restores focus to the trigger on close.
              </Dialog.Description>
              <Dialog.Close style={{ ...controlStyle, alignSelf: "flex-end" }}>
                <Text style={{ fontSize: 12, color: ink }}>Close</Text>
              </Dialog.Close>
            </Dialog.Popup>
          </Dialog.Portal>
        </Dialog.Root>

        <Button
          style={controlStyle}
          onClick={() => {
            nextToast += 1;
            const id = `toast-${nextToast}`;
            setToasts((queue) => [
              ...queue,
              { id, title: `Build ${nextToast} finished`, description: "Auto-dismisses in 4s", duration: 4_000 },
            ]);
          }}
        >
          <Text style={{ fontSize: 12, color: ink }}>Push toast</Text>
        </Button>
      </Row>

      <Toast.Viewport
        scope="toasts"
        toasts={toasts()}
        onDismiss={(ids) =>
          setToasts((queue) => queue.filter((toast) => !ids.includes(toast.id)))
        }
        style={{ display: "flex", flexDirection: "column", gap: 6 }}
      >
        <For each={toasts()}>
          {(toast) => (
            <Toast.Root
              scope="toasts"
              toastId={toast.id}
              style={{
                display: "flex",
                flexDirection: "row",
                alignItems: "center",
                gap: 10,
                padding: 10,
                borderRadius: 10,
                background: "linear-gradient(90deg, #1e293b, #0f172a)",
                borderColor: border,
                borderWidth: 1,
              }}
            >
              <View style={{ display: "flex", flexDirection: "column", flex: 1, gap: 2 }}>
                <Toast.Title scope="toasts" toastId={toast.id}>
                  <Text style={{ fontSize: 12, fontWeight: 700, color: ink }}>{toast.title}</Text>
                </Toast.Title>
                <Toast.Description scope="toasts" toastId={toast.id}>
                  <Text style={{ fontSize: 11, color: muted }}>{toast.description}</Text>
                </Toast.Description>
              </View>
              <Toast.Close scope="toasts" toastId={toast.id} style={{ ...controlStyle, width: 28 }}>
                <Text style={{ fontSize: 12, color: ink }}>×</Text>
              </Toast.Close>
            </Toast.Root>
          )}
        </For>
      </Toast.Viewport>
    </Panel>
  );
}

function ComponentsExample() {
  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        width: "100%",
        height: "100%",
        backgroundColor: "#0b0f17",
        color: ink,
      }}
    >
      <View
        style={{
          height: 52,
          flexShrink: 0,
          justifyContent: "center",
          paddingLeft: 96,
          appRegion: "drag",
        }}
      >
        <Text style={{ fontSize: 14, fontWeight: 700 }}>Declared components</Text>
      </View>
      <View
        style={{
          flex: 1,
          overflowY: "scroll",
          padding: 20,
          display: "grid",
          gridTemplateColumns: 2,
          gap: 16,
        }}
      >
        <OptionSources />
        <RangeAndOrdering />
        <VirtualTable />
        <LazyTree />
        <DateAndTime />
        <Menus />
        <SurfacesAndFeedback />
      </View>
    </View>
  );
}

function openMainWindow() {
  new Window({
    title: "QuickGUI Solid Components",
    width: 1040,
    height: 760,
    minimumWidth: 820,
    minimumHeight: 600,
    background: "#0b0f17",
    titleBarStyle: "hiddenInset",
    trafficLightPosition: { x: 16, y: 18 },
    renderer: createRenderer(() => <ComponentsExample />),
  });
}

app.on("reopen", ({ hasVisibleWindows }) => {
  if (!hasVisibleWindows) openMainWindow();
});
openMainWindow();
