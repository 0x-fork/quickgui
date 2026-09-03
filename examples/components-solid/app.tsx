import { app, Window } from "@quickgui/native";
import {
  Autocomplete,
  Avatar,
  Button,
  Calendar,
  Checkbox,
  CheckboxGroup,
  Combobox,
  ContextMenu,
  DateField,
  Dialog,
  Drawer,
  Menu,
  Menubar,
  NavigationMenu,
  NumberField,
  OtpField,
  Popover,
  PopoverMenu,
  PreviewCard,
  Progress,
  ScrollArea,
  Select,
  Separator,
  Slider,
  Splitter,
  Table,
  Tabs,
  Text,
  TimeField,
  Toast,
  ToggleGroup,
  Toolbar,
  Tooltip,
  Tree,
  View,
  useComboboxChips,
  useComboboxState,
  useGaugeState,
  useMenuItemState,
  useNumberFieldState,
  useSelectState,
  usePopoverPlacement,
  useSliderState,
  useTabsState,
  useToastManager,
  type ScrollAreaState,
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


/**
 * The Base UI-shaped menu compound: rows are application-styled child nodes, and the core owns
 * their identity, roving highlight, toggle policy, radio exclusivity, activation, and closing.
 */
function AlignedMenu() {
  const [open, setOpen] = createSignal(false);
  const [wrap, setWrap] = createSignal(false);
  const [density, setDensity] = createSignal("cozy");
  const [activated, setActivated] = createSignal("nothing yet");
  const rowStyle = {
    display: "flex" as const,
    flexDirection: "row" as const,
    alignItems: "center" as const,
    justifyContent: "space-between" as const,
    gap: 12,
    paddingLeft: 10,
    paddingRight: 10,
    height: 26,
    borderRadius: 6,
  };

  return (
    <Panel title="Base UI menu parts">
      <Menu.Root
        open={open()}
        onOpenChange={setOpen}
        side="bottom"
        align="start"
        sideOffset={6}
      >
        <Menu.Trigger style={controlStyle}>
          <Text style={{ fontSize: 12, color: ink }}>Edit</Text>
        </Menu.Trigger>
        <Menu.Positioner>
          <Menu.Popup
            style={{
              display: "flex",
              flexDirection: "column",
              width: 240,
              padding: 4,
              gap: 2,
              borderRadius: 8,
              borderWidth: 1,
              borderColor: border,
              backgroundColor: "#101725",
            }}
          >
            <Menu.GroupLabel
              value="clipboard"
              label="Clipboard"
              style={{ paddingLeft: 10, height: 20, justifyContent: "center" }}
            >
              <Text style={{ fontSize: 11, color: muted }}>Clipboard</Text>
            </Menu.GroupLabel>
            <MenuRow value="copy" label="Copy" onActivate={setActivated} />
            <Menu.LinkItem
              value="docs"
              label="Documentation"
              href="https://quickgui.dev"
              style={rowStyle}
              onNavigate={(href) => setActivated(`link ${href}`)}
            >
              <Text style={{ fontSize: 12, color: ink }}>Documentation</Text>
            </Menu.LinkItem>
            <Menu.Separator style={{ height: 1, backgroundColor: border, marginTop: 4, marginBottom: 4 }} />
            <Menu.CheckboxItem
              value="wrap"
              label="Wrap lines"
              checked={wrap()}
              onCheckedChange={setWrap}
              style={rowStyle}
            >
              <Text style={{ fontSize: 12, color: ink }}>Wrap lines</Text>
              <Menu.CheckboxItemIndicator>
                <Text style={{ fontSize: 12, color: accent }}>{wrap() ? "✓" : ""}</Text>
              </Menu.CheckboxItemIndicator>
            </Menu.CheckboxItem>
            <Menu.RadioGroup name="density" value={density()} onValueChange={setDensity}>
              <For each={["compact", "cozy"]}>
                {(value) => (
                  <Menu.RadioItem value={value} label={value} style={rowStyle}>
                    <Text style={{ fontSize: 12, color: ink }}>{value}</Text>
                    <Menu.RadioItemIndicator>
                      <Text style={{ fontSize: 12, color: accent }}>
                        {density() === value ? "●" : ""}
                      </Text>
                    </Menu.RadioItemIndicator>
                  </Menu.RadioItem>
                )}
              </For>
            </Menu.RadioGroup>
            <Menu.SubmenuRoot>
              <Menu.SubmenuTrigger
                value="recent"
                label="Open recent"
                openOnHover
                style={rowStyle}
              >
                <Text style={{ fontSize: 12, color: ink }}>Open recent</Text>
                <Text style={{ fontSize: 12, color: muted }}>›</Text>
              </Menu.SubmenuTrigger>
              <Menu.Positioner side="right" align="start" sideOffset={4}>
                <Menu.Popup
                  style={{
                    display: "flex",
                    flexDirection: "column",
                    width: 200,
                    padding: 4,
                    borderRadius: 8,
                    borderWidth: 1,
                    borderColor: border,
                    backgroundColor: "#101725",
                  }}
                >
                  <MenuRow value="notes" label="notes.md" onActivate={setActivated} />
                  <MenuRow value="readme" label="README.md" onActivate={setActivated} />
                </Menu.Popup>
              </Menu.Positioner>
            </Menu.SubmenuRoot>
          </Menu.Popup>
        </Menu.Positioner>
      </Menu.Root>

      <ContextMenu.Root scope="canvas" onSelect={(details) => setActivated(`context ${details.id}`)}>
        <ContextMenu.Trigger
          style={{
            height: 56,
            borderRadius: 10,
            borderWidth: 1,
            borderColor: border,
            borderStyle: "dashed",
            alignItems: "center",
            justifyContent: "center",
          }}
        >
          <Menu.Item value="cut" label="Cut" />
          <Menu.Item value="copy" label="Copy" />
          <Menu.Separator />
          <Menu.Item value="paste" label="Paste" />
          <Text style={{ fontSize: 12, color: muted }}>Right-click: the same row parts</Text>
        </ContextMenu.Trigger>
      </ContextMenu.Root>

      <Text style={{ fontSize: 12, color: muted }}>
        {`activated ${activated()} · wrap ${wrap()} · density ${density()}`}
      </Text>
    </Panel>
  );
}

/** One command row that styles itself from the state the core reports for it. */
function MenuRow(props: { value: string; label: string; onActivate: (value: string) => void }) {
  return (
    <Menu.Item
      value={props.value}
      label={props.label}
      onClick={() => props.onActivate(props.value)}
      style={{
        display: "flex",
        flexDirection: "row",
        alignItems: "center",
        paddingLeft: 10,
        paddingRight: 10,
        height: 26,
        borderRadius: 6,
      }}
    >
      <MenuRowLabel label={props.label} />
    </Menu.Item>
  );
}

function MenuRowLabel(props: { label: string }) {
  const state = useMenuItemState();
  return (
    <Text style={{ fontSize: 12, color: state().highlighted ? "#ffffff" : ink }}>
      {props.label}
    </Text>
  );
}

/** The Base UI-shaped select and combobox parts, and the state the core publishes for them. */
function AlignedPickerParts() {
  const [sizes, setSizes] = createSignal<readonly string[]>(["m"]);
  const [tags, setTags] = createSignal<readonly string[]>(["rust"]);
  const appearance = {
    width: 220,
    rowHeight: 28,
    background: "#101725",
    color: ink,
    highlightBackground: accent,
    highlightColor: "#ffffff",
    mutedColor: muted,
  } as const;

  return (
    <Panel title="Base UI select and combobox parts">
      <Row>
        <Select.Root
          scope="sizes"
          ariaLabel="Sizes"
          multiple
          values={sizes()}
          onValuesChange={setSizes}
          alignItemWithTrigger
          appearance={appearance}
          items={{ s: "Small", m: "Medium", l: "Large" }}
          style={{ ...controlStyle, width: 200 }}
        >
          <Select.Value>
            <SelectValueText />
          </Select.Value>
          <Select.Icon>
            <Text style={{ fontSize: 12, color: muted }}>▾</Text>
          </Select.Icon>
          <Select.Positioner side="bottom" align="start" sideOffset={6}>
            <Select.ScrollUpArrow />
            <Select.ScrollDownArrow />
          </Select.Positioner>
        </Select.Root>

        <Combobox.Root
          scope="tags"
          ariaLabel="Tags"
          multiple
          values={tags()}
          onValuesChange={setTags}
          filterMode="startsWith"
          autoHighlight
          placeholder="Tags"
          appearance={appearance}
          items={[
            { value: "rust", label: "Rust" },
            { value: "zig", label: "Zig" },
            { value: "swift", label: "Swift" },
          ]}
          style={{ ...controlStyle, width: 200, justifyContent: "flex-start" }}
        />
      </Row>
      <ComboboxChips />
      <SelectStateLine />
    </Panel>
  );
}

function SelectValueText() {
  const state = useSelectState();
  return (
    <Text style={{ fontSize: 12, color: state().placeholder ? muted : ink }}>
      {state().placeholder ? "Pick sizes" : "Selected"}
    </Text>
  );
}

function ComboboxChips() {
  const chips = useComboboxChips();
  return (
    <Combobox.Chips style={{ display: "flex", flexDirection: "row", gap: 6 }}>
      <For each={chips()}>
        {(chip, index) => (
          <Combobox.Chip
            index={index()}
            style={{
              display: "flex",
              flexDirection: "row",
              alignItems: "center",
              gap: 4,
              paddingLeft: 8,
              paddingRight: 6,
              height: 22,
              borderRadius: 11,
              backgroundColor: "#1b2436",
            }}
          >
            <Text style={{ fontSize: 11, color: ink }}>{chip.label}</Text>
            <Combobox.ChipRemove index={index()} ariaLabel={`Remove ${chip.label}`}>
              <Text style={{ fontSize: 11, color: muted }}>×</Text>
            </Combobox.ChipRemove>
          </Combobox.Chip>
        )}
      </For>
    </Combobox.Chips>
  );
}

function SelectStateLine() {
  const select = useSelectState();
  const combobox = useComboboxState();
  return (
    <Text style={{ fontSize: 12, color: muted }}>
      {`select filled ${select().filled} · touched ${select().touched} · combobox ${combobox().status}`}
    </Text>
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
          <Tabs.Tab value="overview" index={0} style={controlStyle}>
            <Text style={{ fontSize: 12, color: ink }}>Overview</Text>
          </Tabs.Tab>
          <Tabs.Tab value="usage" index={1} style={controlStyle}>
            <Text style={{ fontSize: 12, color: ink }}>Usage</Text>
          </Tabs.Tab>
          {/* A declared placement keeps the indicator on the tab that is really active and
              publishes that tab's laid-out box back through `useTabsState`. */}
          <Tabs.Indicator
            placement="bottom"
            style={{ height: 2, backgroundColor: accent }}
          />
        </Tabs.List>
        <TabsDirection />
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
        <IdentityAndGrouping />
        <CodesSheetsAndNavigation />
        <AlignedPopoverAndTooltip />
        <AlignedRangeParts />
        <AlignedToastStack />
        <AlignedMenu />
        <AlignedPickerParts />
      </View>
    </View>
  );
}

/** Separators, avatars, checkbox groups, and preview cards: identity, grouping, and hover. */
function IdentityAndGrouping() {
  const allColors = ["red", "green", "blue"] as const;
  const [colors, setColors] = createSignal<readonly string[]>(["green"]);
  const [status, setStatus] = createSignal("idle");
  const [cardOpen, setCardOpen] = createSignal(false);

  return (
    <Panel title="Separator, avatar, checkbox group, preview card">
      <Row>
        <Avatar.Root
          ariaLabel="Ada Lovelace"
          onLoadingStatusChange={setStatus}
          style={{
            width: 40,
            height: 40,
            borderRadius: 20,
            backgroundColor: "#1b2434",
            alignItems: "center",
            justifyContent: "center",
            overflow: "hidden",
          }}
        >
          {/* The core mounts exactly one of these, so the name is announced once. */}
          <Avatar.Image src="./avatar.png" style={{ width: 40, height: 40 }} />
          <Avatar.Fallback delay={120}>
            <Text style={{ fontSize: 13, fontWeight: 700, color: ink }}>AL</Text>
          </Avatar.Fallback>
        </Avatar.Root>
        <Text style={{ fontSize: 12, color: muted }}>status: {status()}</Text>
        <Separator.Root
          orientation="vertical"
          style={{ width: 1, height: 28, backgroundColor: border }}
        />
        <PreviewCard.Root
          open={cardOpen()}
          onOpenChange={setCardOpen}
          placement="bottom-start"
          gap={8}
        >
          <PreviewCard.Trigger delay={400} closeDelay={200} style={controlStyle}>
            <Text style={{ fontSize: 12, color: accent }}>@ada</Text>
          </PreviewCard.Trigger>
          <PreviewCard.Positioner>
            <PreviewCard.Popup
              style={{
                width: 220,
                gap: 6,
                padding: 12,
                borderRadius: 12,
                backgroundColor: surface,
                borderColor: border,
                borderWidth: 1,
                display: "flex",
                flexDirection: "column",
              }}
            >
              <Text style={{ fontSize: 13, fontWeight: 700, color: ink }}>
                Ada Lovelace
              </Text>
              <Text style={{ fontSize: 12, color: muted }}>
                Opens on the core's exact hover deadline, never a JavaScript timer.
              </Text>
            </PreviewCard.Popup>
          </PreviewCard.Positioner>
        </PreviewCard.Root>
      </Row>

      <Separator.Root style={{ height: 1, backgroundColor: border }} />

      <CheckboxGroup.Root
        allValues={[...allColors]}
        value={colors()}
        onValueChange={setColors}
        style={{ display: "flex", flexDirection: "column", gap: 6 }}
      >
        <Row>
          {/* The parent has no retained value: its mixed state is derived from the children. */}
          <Checkbox.Root parent style={controlStyle}>
            <Text style={{ fontSize: 12, color: ink }}>All colours</Text>
          </Checkbox.Root>
        </Row>
        <Row>
          <For each={allColors}>
            {(value) => (
              <Checkbox.Root value={value} style={controlStyle}>
                <Text style={{ fontSize: 12, color: ink }}>{value}</Text>
                <Checkbox.Indicator
                  style={{ marginLeft: 6, width: 8, height: 8, borderRadius: 4 }}
                />
              </Checkbox.Root>
            )}
          </For>
        </Row>
        <Text style={{ fontSize: 12, color: muted }}>
          checked: {colors().join(", ") || "none"}
        </Text>
      </CheckboxGroup.Root>
    </Panel>
  );
}

/** Scroll areas, OTP fields, drawers, and navigation menus. */
function CodesSheetsAndNavigation() {
  const rows = Array.from({ length: 24 }, (_, index) => `Log line ${index + 1}`);
  const rowHeight = 22;
  const [scroll, setScroll] = createSignal<ScrollAreaState | undefined>();
  const [code, setCode] = createSignal("");
  const [completed, setCompleted] = createSignal("");
  const [sheetOpen, setSheetOpen] = createSignal(false);
  const [swipe, setSwipe] = createSignal({ swiping: false, swipeOffset: 0 });
  const [navValue, setNavValue] = createSignal("none");
  const offset = () => scroll()?.offset.y ?? 0;

  return (
    <Panel title="Scroll area, OTP field, drawer, navigation menu">
      <ScrollArea.Root
        viewportSize={{ width: 260, height: 120 }}
        contentSize={{ width: 260, height: rows.length * rowHeight }}
        onScrollStateChange={setScroll}
        style={{ display: "flex", flexDirection: "row", gap: 4, height: 120 }}
      >
        <ScrollArea.Viewport
          style={{ width: 260, height: 120, backgroundColor: "#101725", borderRadius: 8 }}
        >
          {/* Scrolling is a paint-only transform the application applies. */}
          <ScrollArea.Content
            style={{
              display: "flex",
              flexDirection: "column",
              paddingLeft: 8,
              transform: `translateY(${-offset()}px)`,
            }}
          >
            <For each={rows}>
              {(row) => (
                <Text style={{ fontSize: 12, color: muted, height: rowHeight }}>
                  {row}
                </Text>
              )}
            </For>
          </ScrollArea.Content>
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar
          orientation="vertical"
          style={{ width: 6, height: 120, backgroundColor: "#0f172a", borderRadius: 3 }}
        >
          <ScrollArea.Thumb
            style={{
              width: 6,
              height: 40,
              borderRadius: 3,
              backgroundColor: scroll()?.scrolling ? accent : border,
            }}
          />
        </ScrollArea.Scrollbar>
      </ScrollArea.Root>

      <OtpField.Root
        length={6}
        value={code()}
        onValueChange={setCode}
        onComplete={setCompleted}
        style={{ display: "flex", flexDirection: "row", gap: 6 }}
      >
        <For each={[0, 1, 2, 3, 4, 5]}>
          {(index) => (
            <>
              <OtpField.Input
                index={index}
                style={{ ...controlStyle, width: 34, paddingLeft: 0, paddingRight: 0 }}
              />
              {index === 2 ? (
                <OtpField.Separator index={index}>
                  <Text style={{ fontSize: 13, color: muted }}>–</Text>
                </OtpField.Separator>
              ) : null}
            </>
          )}
        </For>
      </OtpField.Root>
      <Text style={{ fontSize: 12, color: muted }}>
        completed: {completed() || "not yet"}
      </Text>

      <Row>
        <Drawer.Root
          open={sheetOpen()}
          onOpenChange={setSheetOpen}
          swipeDirection="down"
          snapPoints={[0.45, 1]}
          onSwipeChange={setSwipe}
        >
          <Drawer.Trigger style={controlStyle}>
            <Text style={{ fontSize: 12, color: ink }}>Open drawer</Text>
          </Drawer.Trigger>
          <Drawer.Portal>
            <Drawer.Backdrop
              style={{
                position: "absolute",
                top: 0,
                right: 0,
                bottom: 0,
                left: 0,
                backgroundColor: "#0f172acc",
              }}
            />
            <Drawer.Viewport
              style={{ display: "flex", flexDirection: "column", justifyContent: "end" }}
            >
              <Drawer.Popup
                style={{
                  gap: 10,
                  padding: 20,
                  borderTopLeftRadius: 16,
                  borderTopRightRadius: 16,
                  backgroundColor: surface,
                  borderColor: border,
                  borderWidth: 1,
                  display: "flex",
                  flexDirection: "column",
                  transform: `translateY(${swipe().swipeOffset}px)`,
                }}
              >
                <Drawer.SwipeArea
                  style={{
                    width: 40,
                    height: 4,
                    borderRadius: 2,
                    alignSelf: "center",
                    backgroundColor: border,
                  }}
                />
                <Drawer.Title>
                  <Text style={{ fontSize: 14, fontWeight: 700, color: ink }}>Filters</Text>
                </Drawer.Title>
                <Drawer.Description>
                  <Text style={{ fontSize: 12, color: muted }}>Narrow the results.</Text>
                </Drawer.Description>
                <Drawer.Content style={{ height: 60 }} />
                <Drawer.Close style={controlStyle}>
                  <Text style={{ fontSize: 12, color: ink }}>Close</Text>
                </Drawer.Close>
              </Drawer.Popup>
            </Drawer.Viewport>
          </Drawer.Portal>
        </Drawer.Root>
      </Row>

      <NavigationMenu.Root
        onValueChange={(next) => setNavValue(next ?? "none")}
      >
        <NavigationMenu.List style={{ display: "flex", flexDirection: "row", gap: 6 }}>
          <For each={["products", "solutions"]}>
            {(item) => (
              <NavigationMenu.Item value={item}>
                <NavigationMenu.Trigger style={controlStyle}>
                  <Text style={{ fontSize: 12, color: ink }}>{item}</Text>
                </NavigationMenu.Trigger>
                <NavigationMenu.Positioner>
                  <NavigationMenu.Popup
                    style={{
                      width: 200,
                      padding: 12,
                      borderRadius: 12,
                      backgroundColor: surface,
                      borderColor: border,
                      borderWidth: 1,
                    }}
                  >
                    <NavigationMenu.Content>
                      <NavigationMenu.Link value={`${item}-home`} active>
                        <Text style={{ fontSize: 12, color: accent }}>{item} home</Text>
                      </NavigationMenu.Link>
                    </NavigationMenu.Content>
                  </NavigationMenu.Popup>
                </NavigationMenu.Positioner>
              </NavigationMenu.Item>
            )}
          </For>
        </NavigationMenu.List>
      </NavigationMenu.Root>
      <Text style={{ fontSize: 12, color: muted }}>open panel: {navValue()}</Text>
    </Panel>
  );
}

/**
 * The Base UI-aligned popover and tooltip compounds.
 *
 * The declared side is only a preference: the core reports the placement it really used, and this
 * panel prints it rather than measuring anything itself.
 */
function AlignedPopoverAndTooltip() {
  const [open, setOpen] = createSignal(false);
  return (
    <Panel title="Popover and tooltip parts">
      <Popover.Root
        open={open()}
        onOpenChange={setOpen}
        side="bottom"
        align="start"
        sideOffset={8}
        collisionPadding={12}
      >
        <Row>
          <Popover.Trigger
            style={{
              paddingLeft: 12,
              paddingRight: 12,
              paddingTop: 6,
              paddingBottom: 6,
              borderRadius: 8,
              backgroundColor: accent,
            }}
          >
            <Text style={{ fontSize: 12 }}>Account</Text>
          </Popover.Trigger>
          <ResolvedPlacement />
        </Row>
        <Popover.Positioner>
          <Popover.Popup
            style={{
              width: 220,
              padding: 12,
              borderRadius: 10,
              backgroundColor: surface,
              borderColor: border,
              borderWidth: 1,
            }}
          >
            <Popover.Arrow
              style={{ width: 10, height: 10, backgroundColor: surface }}
            />
            <Popover.Title>
              <Text style={{ fontSize: 12, fontWeight: 700 }}>Signed in</Text>
            </Popover.Title>
            <Popover.Viewport style={{ maxHeight: 120 }}>
              <Text style={{ fontSize: 12, color: muted }}>ada@example.com</Text>
            </Popover.Viewport>
            <Popover.Close>
              <Text style={{ fontSize: 12, color: accent }}>Done</Text>
            </Popover.Close>
          </Popover.Popup>
        </Popover.Positioner>
      </Popover.Root>

      <Tooltip.Provider delay={400} closeDelay={120} timeout={400}>
        <Tooltip.Root side="top" sideOffset={7}>
          <Tooltip.Trigger
            style={{
              paddingLeft: 12,
              paddingRight: 12,
              paddingTop: 6,
              paddingBottom: 6,
              borderRadius: 8,
              borderColor: border,
              borderWidth: 1,
            }}
          >
            <Text style={{ fontSize: 12 }}>Hover for a hint</Text>
          </Tooltip.Trigger>
          <Tooltip.Positioner>
            <Tooltip.Popup
              style={{
                paddingLeft: 8,
                paddingRight: 8,
                paddingTop: 4,
                paddingBottom: 4,
                borderRadius: 6,
                backgroundColor: "#0b0f17",
                borderColor: border,
                borderWidth: 1,
              }}
            >
              <Text style={{ fontSize: 11 }}>The core owns every deadline</Text>
              <Tooltip.Arrow
                style={{ width: 8, height: 8, backgroundColor: "#0b0f17" }}
              />
            </Tooltip.Popup>
          </Tooltip.Positioner>
        </Tooltip.Root>
      </Tooltip.Provider>
    </Panel>
  );
}

function ResolvedPlacement() {
  const placement = usePopoverPlacement();
  return (
    <Text style={{ fontSize: 12, color: muted }}>
      resolved: {placement().side}/{placement().align}
    </Text>
  );
}

/** Slider, number-field, and progress parts, all reporting what the core decided. */
function AlignedRangeParts() {
  const [volume, setVolume] = createSignal<readonly number[]>([40]);
  const [committed, setCommitted] = createSignal<number | undefined>();
  const [quantity, setQuantity] = createSignal(8);
  return (
    <Panel title="Range parts and reported state">
      <Slider.Root
        scope="aligned-volume"
        value={volume()}
        min={0}
        max={100}
        step={5}
        format="percent"
        onValueChange={setVolume}
        onValueCommitted={([next]) => setCommitted(next)}
      >
        <Row>
          <Slider.Label scope="aligned-volume">
            <Text style={{ fontSize: 12, color: muted }}>Volume</Text>
          </Slider.Label>
          <SliderReadout />
        </Row>
        <Slider.Control scope="aligned-volume">
          <Slider.Track
            scope="aligned-volume"
            style={{ height: 6, borderRadius: 3, backgroundColor: border }}
          >
            <Slider.Indicator
              scope="aligned-volume"
              style={{
                height: 6,
                borderRadius: 3,
                backgroundColor: accent,
                width: `${volume()[0] ?? 0}%`,
              }}
            />
          </Slider.Track>
        </Slider.Control>
        <Slider.Thumb
          scope="aligned-volume"
          index={0}
          style={{ width: 14, height: 14, borderRadius: 7, backgroundColor: ink }}
        />
      </Slider.Root>
      <Text style={{ fontSize: 12, color: muted }}>
        committed: {committed() === undefined ? "—" : String(committed())}
      </Text>

      <NumberField.Root
        scope="aligned-quantity"
        value={quantity()}
        min={0}
        max={99}
        step={1}
        smallStep={0.5}
        largeStep={10}
        snapOnStep
        required
        onValueChange={(next) => setQuantity(next ?? 0)}
      >
        <NumberField.Group scope="aligned-quantity">
          <Row>
            <NumberField.Decrement scope="aligned-quantity">
              <Text style={{ fontSize: 12 }}>−</Text>
            </NumberField.Decrement>
            <NumberField.Input
              scope="aligned-quantity"
              style={{ width: 64, borderColor: border, borderWidth: 1, borderRadius: 6 }}
            />
            <NumberField.Increment scope="aligned-quantity">
              <Text style={{ fontSize: 12 }}>+</Text>
            </NumberField.Increment>
            <NumberField.ScrubArea
              scope="aligned-quantity"
              style={{
                width: 36,
                height: 24,
                borderRadius: 6,
                borderColor: border,
                borderWidth: 1,
              }}
            >
              <NumberFieldScrubCursor />
            </NumberField.ScrubArea>
          </Row>
        </NumberField.Group>
      </NumberField.Root>

      <Progress.Root scope="aligned-upload" value={3} max={4} format="fraction">
        <Row>
          <Progress.Label scope="aligned-upload">
            <Text style={{ fontSize: 12, color: muted }}>Uploading</Text>
          </Progress.Label>
          <GaugeReadout />
        </Row>
        <Progress.Track
          scope="aligned-upload"
          style={{ height: 6, borderRadius: 3, backgroundColor: border }}
        >
          <Progress.Indicator
            scope="aligned-upload"
            style={{ height: 6, borderRadius: 3, backgroundColor: accent, width: "75%" }}
          />
        </Progress.Track>
      </Progress.Root>
    </Panel>
  );
}

function SliderReadout() {
  const slider = useSliderState();
  return (
    <Text style={{ fontSize: 12, color: slider().dragging ? accent : muted }}>
      {slider().displayValue ?? "—"}
    </Text>
  );
}

function NumberFieldScrubCursor() {
  const field = useNumberFieldState();
  return (
    <NumberField.ScrubAreaCursor
      scope="aligned-quantity"
      style={{
        width: 10,
        height: 10,
        borderRadius: 5,
        backgroundColor: field().scrubbing ? accent : border,
      }}
    />
  );
}

function GaugeReadout() {
  const gauge = useGaugeState();
  return (
    <Text style={{ fontSize: 12, color: muted }}>
      {gauge().displayValue ?? "—"} ({gauge().status})
    </Text>
  );
}

/** The toast provider, its manager, and the stack geometry the core reports. */
function AlignedToastStack() {
  return (
    <Toast.Provider timeout={4000} limit={3} pitch={8} swipeDirection="right">
      <AlignedToastPanel />
    </Toast.Provider>
  );
}

function AlignedToastPanel() {
  const toasts = useToastManager();
  return (
    <Panel title="Toast provider and manager">
      <Row>
        <Button
          style={{ paddingLeft: 10, paddingRight: 10, paddingTop: 4, paddingBottom: 4 }}
          onClick={() => toasts.add({ title: "Saved", type: "success" })}
        >
          <Text style={{ fontSize: 12 }}>Push</Text>
        </Button>
        <Button
          style={{ paddingLeft: 10, paddingRight: 10, paddingTop: 4, paddingBottom: 4 }}
          onClick={() => toasts.closeAll()}
        >
          <Text style={{ fontSize: 12 }}>Clear</Text>
        </Button>
      </Row>
      <Toast.Viewport style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        <For each={toasts.stack()}>
          {(entry) => (
            <Toast.Positioner toastId={entry.id}>
              <Toast.Root
                toastId={entry.id}
                style={{
                  padding: 8,
                  borderRadius: 8,
                  backgroundColor: surface,
                  borderColor: border,
                  borderWidth: 1,
                  opacity: entry.limited ? 0.6 : 1,
                }}
              >
                <Toast.Content toastId={entry.id}>
                  <Row>
                    <Toast.Title toastId={entry.id}>
                      <Text style={{ fontSize: 12 }}>{entry.type}</Text>
                    </Toast.Title>
                    <Text style={{ fontSize: 11, color: muted }}>#{entry.index}</Text>
                    <Toast.Close toastId={entry.id}>
                      <Text style={{ fontSize: 12, color: muted }}>×</Text>
                    </Toast.Close>
                  </Row>
                </Toast.Content>
              </Toast.Root>
            </Toast.Positioner>
          )}
        </For>
      </Toast.Viewport>
    </Panel>
  );
}

function TabsDirection() {
  const tabs = useTabsState();
  return (
    <Text style={{ fontSize: 12, color: muted }}>
      activation direction: {tabs().activationDirection}
      {tabs().indicator ? ` · indicator ${Math.round(tabs().indicator!.width)}px` : ""}
    </Text>
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
