import {
  app,
  Appearance,
  Menu as NativeMenu,
  Window,
  type AppearanceMode,
} from "@quickgui/native";
import {
  Accordion,
  AlertDialog,
  Autocomplete,
  Avatar,
  Button,
  Calendar,
  Checkbox,
  CheckboxGroup,
  Collapsible,
  Combobox,
  ContextMenu,
  DateField,
  Dialog,
  Drawer,
  Field,
  Fieldset,
  Input,
  Menu,
  Menubar,
  Meter,
  NavigationMenu,
  NumberField,
  OtpField,
  Popover,
  PopoverMenu,
  PreviewCard,
  Progress,
  Radio,
  RadioGroup,
  ScrollArea,
  Select,
  Separator,
  Slider,
  Splitter,
  Switch,
  Table,
  Tabs,
  Text,
  TimeField,
  Toast,
  Toggle,
  ToggleGroup,
  Toolbar,
  Tooltip,
  Tree,
  View,
  createRenderer,
  useComboboxChips,
  useComboboxState,
  useDrawerSwipe,
  useGaugeState,
  useMenuItemState,
  useNumberFieldState,
  usePopoverPlacement,
  useScrollAreaState,
  useSelectState,
  useSliderState,
  useTabsState,
  useToastManager,
  type AvatarLoadingStatus,
  type DrawerSwipeState,
  type MenuAppearance,
  type PickerAppearance,
  type ScrollAreaState,
  type TableSortState,
  type TreeNodeDeclaration,
  type VisibleRange,
} from "@quickgui/solid";
import { createSignal, For, Show } from "solid-js";

await app.whenReady();

/* -------------------------------------------------------------------------------------------- *
 * Appearance
 *
 * The window reports which appearance it is actually rendering in. Nothing here hardcodes a
 * palette: `light` is the default, `dark` is used only while the system really is dark, and the
 * signal is written from the appearance callback — never from a component body.
 * -------------------------------------------------------------------------------------------- */

interface Palette {
  window: string;
  sidebar: string;
  panel: string;
  panelAlt: string;
  control: string;
  controlHover: string;
  controlActive: string;
  ink: string;
  muted: string;
  faint: string;
  border: string;
  accent: string;
  /** The accent a filled control shows while hovered, so its text stays readable. */
  accentHover: string;
  onAccent: string;
  selection: string;
  popup: string;
  backdrop: string;
  track: string;
  danger: string;
  dangerHover: string;
}

const lightPalette: Palette = {
  window: "#f4f5f7",
  sidebar: "#ebedf1",
  panel: "#ffffff",
  panelAlt: "#f8f9fb",
  control: "#ffffff",
  controlHover: "#eef1f6",
  controlActive: "#e2e8f4",
  ink: "#131820",
  muted: "#5a6472",
  faint: "#8b94a3",
  border: "#d9dde4",
  accent: "#2563eb",
  accentHover: "#1d4fd7",
  onAccent: "#ffffff",
  selection: "#dbe6fd",
  popup: "#ffffff",
  backdrop: "#1e293b66",
  track: "#e4e7ec",
  danger: "#b42318",
  dangerHover: "#9a1d14",
};

const darkPalette: Palette = {
  window: "#0e1117",
  sidebar: "#131924",
  panel: "#171e2a",
  panelAlt: "#1b2331",
  control: "#1c2432",
  controlHover: "#263042",
  controlActive: "#2f3a50",
  ink: "#e7edf7",
  muted: "#98a4b6",
  faint: "#6e7a8c",
  border: "#2a3446",
  accent: "#5b93f7",
  accentHover: "#7aa7ff",
  onAccent: "#08111f",
  selection: "#1f2d47",
  popup: "#171e2a",
  backdrop: "#010409aa",
  track: "#252f41",
  danger: "#f0736a",
  dangerHover: "#f58c84",
};

const [appearance, setAppearance] = createSignal<AppearanceMode>("light");

/**
 * The window extent the core last reported.
 *
 * QuickGUI has no layout observer at the hosted boundary, so anything that needs the window's own
 * size reads it from the window state. It is written from the window's state callback, never from
 * a component body.
 */
const [viewport, setViewport] = createSignal({ width: 1080, height: 780 });

/** The palette the window's current appearance selects. */
const p = (): Palette => (appearance() === "dark" ? darkPalette : lightPalette);

/* -------------------------------------------------------------------------------------------- *
 * Shared styling
 * -------------------------------------------------------------------------------------------- */

const controlStyle = () =>
  ({
    display: "flex",
    flexDirection: "row",
    alignItems: "center",
    justifyContent: "center",
    gap: 6,
    height: 30,
    paddingLeft: 12,
    paddingRight: 12,
    borderRadius: 8,
    backgroundColor: p().control,
    borderColor: p().border,
    borderWidth: 1,
    color: p().ink,
    fontSize: 13,
    cursor: "default",
    userSelect: "none",
    hoverBackgroundColor: p().controlHover,
    focusOutline: `2px solid ${p().accent}`,
    outlineOffset: 2,
  }) as const;

const inputStyle = () =>
  ({
    height: 30,
    paddingLeft: 10,
    paddingRight: 10,
    borderRadius: 8,
    backgroundColor: p().panelAlt,
    borderColor: p().border,
    borderWidth: 1,
    color: p().ink,
    fontSize: 13,
    focusOutline: `2px solid ${p().accent}`,
    outlineOffset: 2,
  }) as const;

const popupStyle = () =>
  ({
    display: "flex",
    flexDirection: "column",
    gap: 8,
    padding: 14,
    borderRadius: 12,
    backgroundColor: p().popup,
    borderColor: p().border,
    borderWidth: 1,
    boxShadow: `0 18px 40px ${appearance() === "dark" ? "#00000088" : "#0f172a2e"}`,
  }) as const;

const menuRowStyle = () =>
  ({
    display: "flex",
    flexDirection: "row",
    alignItems: "center",
    justifyContent: "space-between",
    gap: 12,
    paddingLeft: 10,
    paddingRight: 10,
    height: 28,
    borderRadius: 6,
  }) as const;

const pickerAppearance = (): PickerAppearance => ({
  width: 240,
  rowHeight: 28,
  radius: 10,
  background: p().popup,
  color: p().ink,
  highlightBackground: p().accent,
  highlightColor: p().onAccent,
  selectedBackground: p().selection,
  mutedColor: p().muted,
});

/** Gauges and sliders declare an explicit track width so the core's arithmetic has a basis. */
const GAUGE_WIDTH = 420;

const menuAppearance = (): MenuAppearance => ({
  width: 220,
  radius: 10,
  background: p().popup,
  color: p().ink,
  highlightBackground: p().accent,
  highlightColor: p().onAccent,
  mutedColor: p().muted,
});

/* -------------------------------------------------------------------------------------------- *
 * Small presentational helpers
 * -------------------------------------------------------------------------------------------- */

function Panel(props: { title: string; hint?: string; children?: unknown }) {
  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        gap: 12,
        padding: 16,
        backgroundColor: p().panel,
        borderColor: p().border,
        borderWidth: 1,
        borderRadius: 12,
      }}
    >
      <Text
        style={{
          color: p().faint,
          fontSize: 11,
          letterSpacing: 0.8,
          textTransform: "uppercase",
          fontWeight: 700,
        }}
      >
        {props.title}
      </Text>
      <Show when={props.hint}>
        <Text style={{ fontSize: 12, color: p().muted, lineHeight: 17 }}>
          {props.hint ?? ""}
        </Text>
      </Show>
      {props.children as never}
    </View>
  );
}

/** Fill the declaring element's box. */
const overlayFill = () =>
  ({
    position: "absolute",
    top: 0,
    right: 0,
    bottom: 0,
    left: 0,
  }) as const;

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

function Col(props: { gap?: number; children?: unknown }) {
  return (
    <View
      style={{ display: "flex", flexDirection: "column", gap: props.gap ?? 8 }}
    >
      {props.children as never}
    </View>
  );
}

/** The state the core reported, printed as text. Every demo ends with one of these. */
function Note(props: { text: string }) {
  return (
    <Text
      style={{
        fontSize: 12,
        color: p().muted,
        fontFamily: "ui-monospace, Menlo, monospace",
        lineHeight: 18,
      }}
    >
      {props.text}
    </Text>
  );
}

function Label(props: { text: string }) {
  return <Text style={{ fontSize: 12, color: p().ink }}>{props.text}</Text>;
}

function Muted(props: { text: string }) {
  return <Text style={{ fontSize: 12, color: p().muted }}>{props.text}</Text>;
}

function Btn(props: {
  label: string;
  onClick: () => void;
  primary?: boolean;
  disabled?: boolean;
}) {
  return (
    <Button
      disabled={props.disabled === true}
      style={{
        ...controlStyle(),
        backgroundColor: props.primary === true ? p().accent : p().control,
        borderColor: props.primary === true ? p().accent : p().border,
        // A filled button darkens its own fill on hover instead of taking the neutral hover
        // background, which would put its light label on a light surface.
        hoverBackgroundColor:
          props.primary === true ? p().accentHover : p().controlHover,
        opacity: props.disabled === true ? 0.5 : 1,
      }}
      onClick={() => props.onClick()}
    >
      <Text
        style={{
          fontSize: 12,
          color: props.primary === true ? p().onAccent : p().ink,
        }}
      >
        {props.label}
      </Text>
    </Button>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Accordion
 * -------------------------------------------------------------------------------------------- */

function AccordionDemo() {
  const entries = [
    {
      value: "declared",
      title: "Everything is declared",
      body: "JavaScript declares values and elements; the Rust core owns roles, focus, and deadlines.",
    },
    {
      value: "panels",
      title: "Closed panels are not mounted",
      body: "A closed Accordion.Panel contributes no layout, paint, input, or accessibility node.",
    },
    {
      value: "heading",
      title: "Heading level is the core's",
      body: "headingLevel is clamped by the core to 1 through 6 and projected on every header.",
    },
  ];
  const [open, setOpen] = createSignal<readonly string[]>(["declared"]);

  return (
    <Panel
      title="Accordion"
      hint="multiple + headingLevel. Click a header; the open value set below is the core's answer."
    >
      <Accordion.Root
        multiple
        headingLevel={3}
        value={open()}
        onValueChange={(next) =>
          setOpen(Array.isArray(next) ? next : next ? [next] : [])
        }
        style={{ display: "flex", flexDirection: "column", gap: 6 }}
      >
        <For each={entries}>
          {(entry, index) => (
            <Accordion.Item
              value={entry.value}
              index={index()}
              style={{
                display: "flex",
                flexDirection: "column",
                borderRadius: 10,
                borderWidth: 1,
                borderColor: p().border,
                backgroundColor: p().panelAlt,
                overflow: "hidden",
              }}
            >
              <Accordion.Header style={{ display: "flex", width: "100%" }}>
                <Accordion.Trigger
                  style={{
                    display: "flex",
                    flexDirection: "row",
                    alignItems: "center",
                    justifyContent: "space-between",
                    width: "100%",
                    height: 34,
                    paddingLeft: 12,
                    paddingRight: 12,
                    // The ring is drawn inside the header with the item's own corner radius, so
                    // it never lands on top of the rounded 1px border around the item.
                    borderRadius: 9,
                    hoverBackgroundColor: p().controlHover,
                    focusOutline: `2px solid ${p().accent}`,
                    outlineOffset: -2,
                  }}
                >
                  <Text style={{ fontSize: 12, color: p().ink }}>
                    {entry.title}
                  </Text>
                  <Text style={{ fontSize: 12, color: p().muted }}>
                    {open().includes(entry.value) ? "−" : "+"}
                  </Text>
                </Accordion.Trigger>
              </Accordion.Header>
              <Accordion.Panel
                style={{ paddingLeft: 12, paddingRight: 12, paddingBottom: 10 }}
              >
                <Text style={{ fontSize: 12, color: p().muted, lineHeight: 17 }}>
                  {entry.body}
                </Text>
              </Accordion.Panel>
            </Accordion.Item>
          )}
        </For>
      </Accordion.Root>
      <Note text={`open: ${open().join(", ") || "none"}`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Alert dialog
 * -------------------------------------------------------------------------------------------- */

function AlertDialogDemo() {
  const [open, setOpen] = createSignal(false);
  const [outcome, setOutcome] = createSignal("nothing yet");
  const [reason, setReason] = createSignal("—");

  return (
    <Panel
      title="Alert dialog"
      hint="An alert dialog refuses a backdrop dismissal by default, so a consequential choice needs a real answer. The portal covers the whole window."
    >
      <AlertDialog.Root
        open={open()}
        onOpenChange={(next, details) => {
          setOpen(next);
          setReason(details.reason);
          if (!next && outcome() === "nothing yet") setOutcome("dismissed");
        }}
      >
        <AlertDialog.Trigger style={controlStyle()}>
          <Text style={{ fontSize: 12, color: p().danger }}>Delete branch…</Text>
        </AlertDialog.Trigger>
        <AlertDialog.Portal
          style={{
            ...overlayFill(),
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
          }}
        >
          <AlertDialog.Backdrop
            style={{ ...overlayFill(), backgroundColor: p().backdrop }}
          />
          <AlertDialog.Popup style={{ ...popupStyle(), width: 340 }}>
              <AlertDialog.Title
                style={{ fontSize: 15, fontWeight: 700, color: p().ink }}
              >
                Delete “electron-parity”?
              </AlertDialog.Title>
              <AlertDialog.Description
                style={{ fontSize: 12, color: p().muted, lineHeight: 17 }}
              >
                The core traps focus here and restores it to the trigger on every dismissal path.
              </AlertDialog.Description>
              <Row>
                <AlertDialog.Close style={controlStyle()}>
                  <Text style={{ fontSize: 12, color: p().ink }}>Cancel</Text>
                </AlertDialog.Close>
                <Button
                  style={{
                    ...controlStyle(),
                    backgroundColor: p().danger,
                    borderColor: p().danger,
                    hoverBackgroundColor: p().dangerHover,
                  }}
                  onClick={() => {
                    setOutcome("deleted");
                    setOpen(false);
                  }}
                >
                  <Text style={{ fontSize: 12, color: "#ffffff" }}>Delete</Text>
                </Button>
              </Row>
          </AlertDialog.Popup>
        </AlertDialog.Portal>
      </AlertDialog.Root>
      <Note text={`open ${open()} · last reason ${reason()} · outcome ${outcome()}`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Autocomplete
 * -------------------------------------------------------------------------------------------- */

function AutocompleteDemo() {
  const [query, setQuery] = createSignal("");
  const [committed, setCommitted] = createSignal("—");
  const [open, setOpen] = createSignal(false);

  return (
    <Panel
      title="Autocomplete"
      hint="Free-form text with suggestions. The core filters, ranks, and paints the rows in its own popover window."
    >
      <Autocomplete.Root
        scope="ac-search"
        ariaLabel="Search components"
        placeholder="Type “win”, “tab”, or “tool”…"
        inputValue={query()}
        filterMode="fuzzy"
        items={[
          { value: "window", label: "Window" },
          { value: "widget", label: "Widget" },
          { value: "tabs", label: "Tabs" },
          { value: "table", label: "Table" },
          { value: "toolbar", label: "Toolbar" },
          { value: "tooltip", label: "Tooltip" },
        ]}
        appearance={pickerAppearance()}
        onInputValueChange={setQuery}
        onOpenChange={setOpen}
        onCommit={(details) =>
          setCommitted(String(details.value ?? details.inputValue ?? "—"))
        }
        style={{ ...inputStyle(), width: 280 }}
      />
      <Note text={`query "${query()}" · popover ${open()} · committed ${committed()}`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Avatar
 * -------------------------------------------------------------------------------------------- */

function AvatarDemo() {
  const [missing, setMissing] = createSignal<AvatarLoadingStatus>("idle");
  const [initials, setInitials] = createSignal<AvatarLoadingStatus>("idle");

  const shell = () =>
    ({
      width: 44,
      height: 44,
      borderRadius: 22,
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      backgroundColor: p().selection,
      borderColor: p().border,
      borderWidth: 1,
      overflow: "hidden",
    }) as const;

  return (
    <Panel
      title="Avatar"
      hint="The core mounts exactly one of the image and the fallback, so the name is announced once. A load failure is reported, never guessed."
    >
      <Row>
        <Avatar.Root
          ariaLabel="Ada Lovelace"
          onLoadingStatusChange={setMissing}
          style={shell()}
        >
          <Avatar.Image src="./avatar-does-not-exist.png" style={{ width: 44, height: 44 }} />
          <Avatar.Fallback delay={120}>
            <Text style={{ fontSize: 14, fontWeight: 700, color: p().ink }}>AL</Text>
          </Avatar.Fallback>
        </Avatar.Root>
        <Avatar.Root
          ariaLabel="Grace Hopper"
          onLoadingStatusChange={setInitials}
          style={shell()}
        >
          <Avatar.Fallback>
            <Text style={{ fontSize: 14, fontWeight: 700, color: p().ink }}>GH</Text>
          </Avatar.Fallback>
        </Avatar.Root>
      </Row>
      <Note text={`with image: ${missing()} · fallback only: ${initials()}`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Button
 * -------------------------------------------------------------------------------------------- */

function ButtonDemo() {
  const [clicks, setClicks] = createSignal(0);
  const [last, setLast] = createSignal("nothing yet");

  return (
    <Panel
      title="Button"
      hint="An ordinary native button node: the core owns the press, the focus ring, the cursor, and the window-drag exclusion."
    >
      <Row>
        <Btn label="Primary" primary onClick={() => { setClicks(clicks() + 1); setLast("primary"); }} />
        <Btn label="Secondary" onClick={() => { setClicks(clicks() + 1); setLast("secondary"); }} />
        <Btn label="Disabled" disabled onClick={() => setLast("never")} />
        <Button
          style={controlStyle()}
          onDoubleClick={() => setLast("double click")}
          onClick={() => setLast("single click")}
        >
          <Text style={{ fontSize: 12, color: p().ink }}>Double-click me</Text>
        </Button>
      </Row>
      <Note text={`clicks ${clicks()} · last ${last()}`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Calendar
 * -------------------------------------------------------------------------------------------- */

function monthGrid(month: string): readonly (readonly string[])[] {
  const parts = month.split("-");
  const year = Number(parts[0] ?? "2026");
  const monthNumber = Number(parts[1] ?? "9");
  const first = new Date(Date.UTC(year, monthNumber - 1, 1));
  const weekday = first.getUTCDay();
  const start = weekday === 0 ? 6 : weekday - 1;
  return Array.from({ length: 6 }, (_, week) =>
    Array.from({ length: 7 }, (_, column) => {
      const date = new Date(first);
      date.setUTCDate(1 - start + week * 7 + column);
      return date.toISOString().slice(0, 10);
    }),
  );
}

function CalendarDemo() {
  const [day, setDay] = createSignal("2026-09-03");
  const [month, setMonth] = createSignal("2026-09");
  const [focused, setFocused] = createSignal("2026-09-03");

  return (
    <Panel
      title="Calendar"
      hint="ISO civil dates with no time zone. The grid is application-declared; the single Tab stop, arrow navigation, and month changes are the core's."
    >
      <Calendar.Root
        scope="cal"
        value={day()}
        min="2026-01-01"
        max="2026-12-31"
        firstWeekday={0}
        onValueChange={(next) => setDay(next ?? "")}
        onMonthChange={setMonth}
        onFocusChange={setFocused}
        style={{ display: "flex", flexDirection: "column", gap: 3 }}
      >
        <View style={{ display: "flex", flexDirection: "row", gap: 3 }}>
          <For each={["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"]}>
            {(name) => (
              <View
                style={{
                  width: 32,
                  height: 20,
                  alignItems: "center",
                  justifyContent: "center",
                }}
              >
                <Text style={{ fontSize: 10, color: p().faint, fontWeight: 700 }}>
                  {name}
                </Text>
              </View>
            )}
          </For>
        </View>
        <For each={monthGrid(month())}>
          {(week, index) => (
            <Calendar.Week
              scope="cal"
              itemIndex={index()}
              style={{ display: "flex", flexDirection: "row", gap: 3 }}
            >
              <For each={week}>
                {(date) => (
                  <Calendar.Day
                    scope="cal"
                    day={date}
                    style={{
                      width: 32,
                      height: 28,
                      borderRadius: 7,
                      alignItems: "center",
                      justifyContent: "center",
                      backgroundColor:
                        date === day() ? p().accent : p().panelAlt,
                      hoverBackgroundColor:
                        date === day() ? p().accent : p().controlHover,
                      focusOutline: `2px solid ${p().accent}`,
                      outlineOffset: 1,
                    }}
                  >
                    <Text
                      style={{
                        fontSize: 11,
                        color:
                          date === day()
                            ? p().onAccent
                            : date.slice(0, 7) === month()
                              ? p().ink
                              : p().faint,
                      }}
                    >
                      {date.slice(8)}
                    </Text>
                  </Calendar.Day>
                )}
              </For>
            </Calendar.Week>
          )}
        </For>
      </Calendar.Root>
      <Note text={`selected ${day()} · showing ${month()} · tab stop ${focused()}`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Checkbox
 * -------------------------------------------------------------------------------------------- */

function CheckboxDemo() {
  const [notify, setNotify] = createSignal<boolean | "indeterminate">(
    "indeterminate",
  );
  const [locked, setLocked] = createSignal(true);
  const [kids, setKids] = createSignal<readonly boolean[]>([true, false]);

  const box = () =>
    ({
      display: "flex",
      flexDirection: "row",
      alignItems: "center",
      gap: 8,
      height: 28,
      paddingLeft: 8,
      paddingRight: 10,
      borderRadius: 8,
      hoverBackgroundColor: p().controlHover,
      focusOutline: `2px solid ${p().accent}`,
    }) as const;

  const tick = (checked: boolean | "indeterminate") =>
    ({
      width: 16,
      height: 16,
      borderRadius: 5,
      alignItems: "center",
      justifyContent: "center",
      borderWidth: 1,
      borderColor: checked === false ? p().border : p().accent,
      backgroundColor: checked === false ? p().control : p().accent,
    }) as const;

  return (
    <Panel
      title="Checkbox"
      hint="Tri-state on/off/mixed, a read-only box that keeps its Tab stop, and a standalone parent whose mixed state the core derives from childrenChecked."
    >
      <Checkbox.Root
        checked={notify()}
        onCheckedChange={setNotify}
        style={box()}
      >
        <Checkbox.Indicator style={tick(notify())}>
          <Text style={{ fontSize: 11, color: p().onAccent }}>
            {notify() === true ? "✓" : notify() === "indeterminate" ? "–" : ""}
          </Text>
        </Checkbox.Indicator>
        <Label text="Email me about releases" />
      </Checkbox.Root>

      <Checkbox.Root checked={locked()} readOnly style={box()}>
        <Checkbox.Indicator style={tick(locked())}>
          <Text style={{ fontSize: 11, color: p().onAccent }}>✓</Text>
        </Checkbox.Indicator>
        <Muted text="Read-only: focusable, refuses changes" />
      </Checkbox.Root>

      <Separator.Root style={{ height: 1, backgroundColor: p().border }} />

      <Checkbox.Root childrenChecked={kids()} style={box()}>
        <Checkbox.Indicator style={tick(kids().every((k) => k) ? true : kids().some((k) => k) ? "indeterminate" : false)} />
        <Label text="Parent, derived from its children" />
      </Checkbox.Root>
      <Row>
        <For each={["Analytics", "Crash reports"]}>
          {(name, index) => (
            <Checkbox.Root
              checked={kids()[index()] === true}
              onCheckedChange={(next) =>
                setKids(kids().map((k, i) => (i === index() ? next : k)))
              }
              style={box()}
            >
              <Checkbox.Indicator style={tick(kids()[index()] === true)} />
              <Label text={name} />
            </Checkbox.Root>
          )}
        </For>
      </Row>
      <Note
        text={`notify ${String(notify())} · locked ${locked()} · children [${kids().join(", ")}]`}
      />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Checkbox group
 * -------------------------------------------------------------------------------------------- */

function CheckboxGroupDemo() {
  const all = ["red", "green", "blue", "violet"];
  const [colors, setColors] = createSignal<readonly string[]>(["green"]);

  const box = () =>
    ({
      display: "flex",
      flexDirection: "row",
      alignItems: "center",
      gap: 8,
      height: 28,
      paddingLeft: 8,
      paddingRight: 10,
      borderRadius: 8,
      hoverBackgroundColor: p().controlHover,
      focusOutline: `2px solid ${p().accent}`,
    }) as const;

  return (
    <Panel
      title="Checkbox group"
      hint="The parent has no retained value of its own; the core folds the children into on, mixed, or off, and keeps checked values in the declared order."
    >
      <CheckboxGroup.Root
        allValues={all}
        value={colors()}
        onValueChange={setColors}
        style={{ display: "flex", flexDirection: "column", gap: 4 }}
      >
        <Checkbox.Root parent style={box()}>
          <Checkbox.Indicator
            style={{
              width: 16,
              height: 16,
              borderRadius: 5,
              borderWidth: 1,
              borderColor: p().accent,
              backgroundColor: colors().length > 0 ? p().accent : p().control,
            }}
          />
          <Label text="All colours" />
        </Checkbox.Root>
        <For each={all}>
          {(value) => (
            <Checkbox.Root value={value} style={{ ...box(), marginLeft: 18 }}>
              <Checkbox.Indicator
                style={{
                  width: 16,
                  height: 16,
                  borderRadius: 5,
                  borderWidth: 1,
                  borderColor: colors().includes(value) ? p().accent : p().border,
                  backgroundColor: colors().includes(value)
                    ? p().accent
                    : p().control,
                }}
              />
              <Label text={value} />
            </Checkbox.Root>
          )}
        </For>
      </CheckboxGroup.Root>
      <Note text={`checked: ${colors().join(", ") || "none"}`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Collapsible
 * -------------------------------------------------------------------------------------------- */

function CollapsibleDemo() {
  const [open, setOpen] = createSignal(false);
  const [kept, setKept] = createSignal(true);

  return (
    <Panel
      title="Collapsible"
      hint="One disclosure. Its panel is omitted entirely while closed unless keepMounted retains it as display:none."
    >
      <Collapsible.Root
        open={open()}
        onOpenChange={setOpen}
        style={{
          display: "flex",
          flexDirection: "column",
          gap: 6,
          borderRadius: 10,
          borderWidth: 1,
          borderColor: p().border,
          padding: 10,
          backgroundColor: p().panelAlt,
        }}
      >
        <Collapsible.Trigger style={controlStyle()}>
          <Text style={{ fontSize: 12, color: p().ink }}>
            {open() ? "Hide advanced options" : "Show advanced options"}
          </Text>
        </Collapsible.Trigger>
        <Collapsible.Panel style={{ paddingTop: 4 }}>
          <Text style={{ fontSize: 12, color: p().muted, lineHeight: 17 }}>
            While closed this panel contributes no layout, paint, input, or accessibility node at all.
          </Text>
        </Collapsible.Panel>
      </Collapsible.Root>

      <Collapsible.Root
        open={kept()}
        onOpenChange={setKept}
        keepMounted
        style={{
          display: "flex",
          flexDirection: "column",
          gap: 6,
          borderRadius: 10,
          borderWidth: 1,
          borderColor: p().border,
          padding: 10,
          backgroundColor: p().panelAlt,
        }}
      >
        <Collapsible.Trigger style={controlStyle()}>
          <Text style={{ fontSize: 12, color: p().ink }}>
            keepMounted · {kept() ? "open" : "closed"}
          </Text>
        </Collapsible.Trigger>
        <Collapsible.Panel style={{ paddingTop: 4 }}>
          <Muted text="Retained as display:none instead of omitted." />
        </Collapsible.Panel>
      </Collapsible.Root>

      <Note text={`plain ${open()} · keepMounted ${kept()}`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Combobox
 * -------------------------------------------------------------------------------------------- */

function ComboboxDemo() {
  const [fruit, setFruit] = createSignal("apple");
  const [tags, setTags] = createSignal<readonly string[]>(["rust"]);

  return (
    <Panel
      title="Combobox"
      hint="A constrained picker with child Option nodes, and a multiple combobox whose chips come back from the core."
    >
      <Combobox.Root
        scope="cb-fruit"
        ariaLabel="Fruit"
        placeholder="Pick a fruit"
        value={fruit()}
        autoHighlight
        filterMode="contains"
        appearance={pickerAppearance()}
        onValueChange={(next) => setFruit(next ?? "")}
        style={{ ...inputStyle(), width: 240 }}
      >
        <Combobox.Option scope="cb-fruit" partValue="apple" label="Apple" group="Common" />
        <Combobox.Option scope="cb-fruit" partValue="banana" label="Banana" group="Common" />
        <Combobox.Option scope="cb-fruit" partValue="lychee" label="Lychee" group="Tropical" />
        <Combobox.Option scope="cb-fruit" partValue="mango" label="Mango" group="Tropical" />
      </Combobox.Root>

      <Combobox.Root
        scope="cb-tags"
        ariaLabel="Tags"
        multiple
        values={tags()}
        onValuesChange={setTags}
        filterMode="startsWith"
        autoHighlight
        placeholder="Add tags"
        appearance={pickerAppearance()}
        items={[
          { value: "rust", label: "Rust" },
          { value: "zig", label: "Zig" },
          { value: "swift", label: "Swift" },
          { value: "typescript", label: "TypeScript" },
        ]}
        style={{
          ...inputStyle(),
          width: 300,
          height: 34,
          display: "flex",
          flexDirection: "row",
          alignItems: "center",
        }}
      >
        <ComboboxChipList />
      </Combobox.Root>
      <ComboboxReadout selected={() => fruit()} chips={() => tags()} />
    </Panel>
  );
}

function ComboboxChipList() {
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
              backgroundColor: p().selection,
            }}
          >
            <Text style={{ fontSize: 11, color: p().ink }}>{chip.label}</Text>
            <Combobox.ChipRemove index={index()} ariaLabel={`Remove ${chip.label}`}>
              <Text style={{ fontSize: 11, color: p().muted }}>×</Text>
            </Combobox.ChipRemove>
          </Combobox.Chip>
        )}
      </For>
    </Combobox.Chips>
  );
}

function ComboboxReadout(props: {
  selected: () => string;
  chips: () => readonly string[];
}) {
  const state = useComboboxState();
  return (
    <Note
      text={`value ${props.selected()} · tags [${props.chips().join(", ")}] · open ${state().popupOpen} · results ${state().resultCount}`}
    />
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Context menu
 * -------------------------------------------------------------------------------------------- */

function ContextMenuDemo() {
  const [command, setCommand] = createSignal("nothing yet");
  const [parts, setParts] = createSignal("nothing yet");
  const [native, setNative] = createSignal("nothing yet");

  const target = () =>
    ({
      height: 64,
      borderRadius: 10,
      borderWidth: 1,
      borderColor: p().border,
      borderStyle: "dashed",
      alignItems: "center",
      justifyContent: "center",
      backgroundColor: p().panelAlt,
    }) as const;

  return (
    <Panel
      title="Context menu"
      hint="Three ways to answer a secondary click: the core's in-window surface from a declared JSON row model, the same surface from Base UI row parts, and the operating system's own menu through Menu.popup from @quickgui/native."
    >
      <ContextMenu.Root
        scope="ctx-json"
        items={[
          { id: "cut", label: "Cut", shortcut: "⌘X" },
          { id: "copy", label: "Copy", shortcut: "⌘C" },
          { type: "separator" },
          { id: "paste", label: "Paste", shortcut: "⌘V", disabled: true },
        ]}
        appearance={menuAppearance()}
        onSelect={(details) => setCommand(details.id)}
      >
        <ContextMenu.Trigger style={target()}>
          <Muted text="Right-click: declared JSON rows" />
        </ContextMenu.Trigger>
      </ContextMenu.Root>

      {/* The child parts are the rows; the surface they sit on is still the declared appearance. */}
      <ContextMenu.Root
        scope="ctx-parts"
        appearance={menuAppearance()}
        onSelect={(details) => setParts(details.id)}
      >
        <ContextMenu.Trigger style={target()}>
          <Menu.Item value="rename" label="Rename" />
          <Menu.Item value="duplicate" label="Duplicate" />
          <Menu.Separator />
          <Menu.Item value="delete" label="Delete" />
          <Muted text="Right-click: the same Menu.Item parts" />
        </ContextMenu.Trigger>
      </ContextMenu.Root>

      {/* The system menu is AppKit's own NSMenu: it opens at the cursor and resolves on close. */}
      <View
        style={{ ...target(), display: "flex" }}
        onContextMenu={() => {
          void NativeMenu.popup([
            { label: "Reveal in Finder", click: () => setNative("reveal") },
            { label: "Get Info", click: () => setNative("info") },
            { type: "separator" },
            { label: "Move to Trash", click: () => setNative("trash") },
          ]).then(() => {
            if (native() === "nothing yet") setNative("dismissed");
          });
        }}
      >
        <Muted text="Right-click: the system's native menu" />
      </View>

      <Note text={`json → ${command()} · parts → ${parts()} · native → ${native()}`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Date field
 * -------------------------------------------------------------------------------------------- */

function DateFieldDemo() {
  const [due, setDue] = createSignal("2026-09-03");
  const [iso, setIso] = createSignal("2026-01-15");

  const segment = () => ({ ...inputStyle(), width: 46, textAlign: "center" }) as const;

  return (
    <Panel
      title="Date field"
      hint="Segmented civil dates. Up/Down step a segment, typing rolls into the next one, and the core clamps into min/max before reporting."
    >
      <Row>
        <Muted text="mdy" />
        <DateField.Root
          scope="df-due"
          value={due()}
          format="mdy"
          min="2026-01-01"
          max="2026-12-31"
          onValueChange={(next) => setDue(next ?? "")}
          style={{ display: "flex", flexDirection: "row", gap: 4 }}
        >
          <DateField.Segment scope="df-due" segment="month" style={segment()} />
          <DateField.Segment scope="df-due" segment="day" style={segment()} />
          <DateField.Segment
            scope="df-due"
            segment="year"
            style={{ ...segment(), width: 68 }}
          />
        </DateField.Root>
      </Row>
      <Row>
        <Muted text="ymd" />
        <DateField.Root
          scope="df-iso"
          value={iso()}
          onValueChange={(next) => setIso(next ?? "")}
          style={{ display: "flex", flexDirection: "row", gap: 4 }}
        >
          <DateField.Segment
            scope="df-iso"
            segment="year"
            style={{ ...segment(), width: 68 }}
          />
          <DateField.Segment scope="df-iso" segment="month" style={segment()} />
          <DateField.Segment scope="df-iso" segment="day" style={segment()} />
        </DateField.Root>
      </Row>
      <Note text={`due ${due()} · iso ${iso()}`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Dialog
 * -------------------------------------------------------------------------------------------- */

function DialogDemo() {
  const [open, setOpen] = createSignal(false);
  const [reason, setReason] = createSignal("—");
  const [completed, setCompleted] = createSignal("—");
  const [notes, setNotes] = createSignal("");

  return (
    <Panel
      title="Dialog"
      hint="An in-window dialog with the core's own focus trap. exitDuration is the exact deadline the surface stays mounted for, and Dialog.Viewport is the scrollable body."
    >
      <Dialog.Root
        open={open()}
        exitDuration={120}
        onOpenChange={(next, details) => {
          setOpen(next);
          setReason(details.reason);
        }}
        onOpenChangeComplete={(next) =>
          setCompleted(next ? "opened" : "closed")
        }
      >
        <Dialog.Trigger style={controlStyle()}>
          <Text style={{ fontSize: 12, color: p().ink }}>Open dialog</Text>
        </Dialog.Trigger>
        <Dialog.Portal
          style={{
            ...overlayFill(),
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
          }}
        >
          <Dialog.Backdrop
            style={{ ...overlayFill(), backgroundColor: p().backdrop }}
          />
          <Dialog.Popup style={{ ...popupStyle(), width: 360 }}>
            <Dialog.Title style={{ fontSize: 15, fontWeight: 700, color: p().ink }}>
              Publish this build?
            </Dialog.Title>
            <Dialog.Description
              style={{ fontSize: 12, color: p().muted, lineHeight: 17 }}
            >
              Escape, the backdrop, and the close control all restore focus to the trigger.
            </Dialog.Description>
            {/* Dialog.Viewport is the scrollable dialog body: the core owns its overflow. */}
            <Dialog.Viewport
              style={{
                display: "flex",
                flexDirection: "column",
                gap: 8,
                maxHeight: 160,
              }}
            >
              <Input
                value={notes()}
                multiline
                placeholder="Release notes"
                onInput={(event) => setNotes(event.value ?? "")}
                style={{ ...inputStyle(), width: 320, height: 64, paddingTop: 6 }}
              />
            </Dialog.Viewport>
            <Row>
              <Dialog.Close style={controlStyle()}>
                <Text style={{ fontSize: 12, color: p().ink }}>Cancel</Text>
              </Dialog.Close>
              <Btn label="Publish" primary onClick={() => setOpen(false)} />
            </Row>
          </Dialog.Popup>
        </Dialog.Portal>
      </Dialog.Root>
      <Note
        text={`open ${open()} · reason ${reason()} · transition ${completed()} · notes ${notes().length} chars`}
      />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Drawer
 * -------------------------------------------------------------------------------------------- */


function DrawerDemo() {
  const [open, setOpen] = createSignal(false);
  const [snap, setSnap] = createSignal(0);
  const [swipe, setSwipe] = createSignal<DrawerSwipeState>({
    swiping: false,
    swipeOffset: 0,
  });

  return (
    <Panel
      title="Drawer"
      hint="A bottom sheet with bounded snap points over the whole window. The swipe offset is a paint-only transform the application applies; the flick velocity that dismisses is the core's."
    >
      <Drawer.Root
        open={open()}
        onOpenChange={setOpen}
        swipeDirection="down"
        snapPoints={[0.4, 0.9]}
        snapPoint={snap()}
        onSnapPointChange={setSnap}
        onSwipeChange={setSwipe}
      >
        <Drawer.Trigger style={controlStyle()}>
          <Text style={{ fontSize: 12, color: p().ink }}>Open drawer</Text>
        </Drawer.Trigger>
        <Drawer.Portal style={overlayFill()}>
          <Drawer.Backdrop
            style={{ ...overlayFill(), backgroundColor: p().backdrop }}
          />
          <Drawer.Viewport
            style={{
              ...overlayFill(),
              display: "flex",
              flexDirection: "column",
              justifyContent: "flex-end",
            }}
          >
            <Drawer.Popup
              style={{
                display: "flex",
                flexDirection: "column",
                gap: 10,
                padding: 20,
                width: "100%",
                borderTopLeftRadius: 18,
                borderTopRightRadius: 18,
                backgroundColor: p().popup,
                borderColor: p().border,
                borderWidth: 1,
                transform: `translateY(${swipe().swipeOffset}px)`,
              }}
            >
              <Drawer.SwipeArea
                style={{
                  width: 44,
                  height: 5,
                  borderRadius: 3,
                  alignSelf: "center",
                  backgroundColor: swipe().swiping ? p().accent : p().border,
                }}
              />
              <Drawer.Title>
                <Text style={{ fontSize: 15, fontWeight: 700, color: p().ink }}>
                  Filters
                </Text>
              </Drawer.Title>
              <Drawer.Description>
                <Muted text="Drag the handle down to snap or dismiss." />
              </Drawer.Description>
              <Drawer.Content style={{ height: 80 }}>
                <DrawerSwipeReadout />
              </Drawer.Content>
              <Drawer.Close style={controlStyle()}>
                <Text style={{ fontSize: 12, color: p().ink }}>Close</Text>
              </Drawer.Close>
            </Drawer.Popup>
          </Drawer.Viewport>
        </Drawer.Portal>
      </Drawer.Root>
      <Note
        text={`open ${open()} · snap point ${snap()} · swiping ${swipe().swiping} · offset ${Math.round(swipe().swipeOffset)}`}
      />
    </Panel>
  );
}

function DrawerSwipeReadout() {
  const swipe = useDrawerSwipe();
  return (
    <Note
      text={`useDrawerSwipe → swiping ${swipe().swiping}, offset ${Math.round(swipe().swipeOffset)}`}
    />
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Field
 * -------------------------------------------------------------------------------------------- */

function FieldDemo() {
  const [email, setEmail] = createSignal("");
  const [triggers, setTriggers] = createSignal("—");
  const valid = () => email().includes("@");

  return (
    <Panel
      title="Field"
      hint="One control plus its label, description, error, and live validity. validationMode and validationDebounceTime are answered by the core through onValidationChange."
    >
      <Field.Root
        invalid={!valid()}
        required
        filled={email().length > 0}
        validationMessage="Enter an address containing @"
        validationMode="onChange"
        validationDebounceTime={200}
        onValidationChange={(validation) =>
          setTriggers(
            `change ${validation.triggers.change} · blur ${validation.triggers.blur} · submit ${validation.triggers.submit} · debounce ${validation.delay.change}ms`,
          )
        }
        style={{ display: "flex", flexDirection: "column", gap: 6 }}
      >
        <Field.Label>
          <Text style={{ fontSize: 12, fontWeight: 700, color: p().ink }}>
            Email
          </Text>
        </Field.Label>
        <Field.Control
          value={email()}
          placeholder="you@example.com"
          onInput={(event) => setEmail(event.value ?? "")}
          style={{ ...inputStyle(), width: 280 }}
        />
        <Field.Description>
          <Muted text="We never share it." />
        </Field.Description>
        <Field.Error>
          <Text style={{ fontSize: 12, color: p().danger }}>
            Enter an address containing @
          </Text>
        </Field.Error>
        <Field.Validity>
          <Muted text={valid() ? "valid" : "invalid"} />
        </Field.Validity>
      </Field.Root>
      <Note text={`value "${email()}" · ${triggers()}`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Fieldset
 * -------------------------------------------------------------------------------------------- */

function FieldsetDemo() {
  const [saving, setSaving] = createSignal(false);
  const [name, setName] = createSignal("Ada");
  const [org, setOrg] = createSignal("Analytical Engines");

  return (
    <Panel
      title="Fieldset"
      hint="A semantic group with a legend. A Field.Root inside it inherits the group's disabled state, so one flag disables the whole section."
    >
      <Row>
        <Switch.Root
          checked={saving()}
          onCheckedChange={setSaving}
          style={{
            width: 40,
            height: 22,
            borderRadius: 11,
            padding: 2,
            backgroundColor: saving() ? p().accent : p().track,
            focusOutline: `2px solid ${p().accent}`,
          }}
        >
          <Switch.Thumb
            style={{
              width: 18,
              height: 18,
              borderRadius: 9,
              backgroundColor: "#ffffff",
              marginLeft: saving() ? 18 : 0,
            }}
          />
        </Switch.Root>
        <Muted text={saving() ? "saving: the fieldset is disabled" : "editable"} />
      </Row>
      <Fieldset.Root
        disabled={saving()}
        style={{
          display: "flex",
          flexDirection: "column",
          gap: 10,
          padding: 14,
          borderRadius: 10,
          borderWidth: 1,
          borderColor: p().border,
          backgroundColor: p().panelAlt,
          opacity: saving() ? 0.55 : 1,
        }}
      >
        <Fieldset.Legend>
          <Text style={{ fontSize: 12, fontWeight: 700, color: p().ink }}>
            Account
          </Text>
        </Fieldset.Legend>
        <Fieldset.Description>
          <Muted text="Both fields inherit the group's disabled state." />
        </Fieldset.Description>
        <Field.Root style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          <Field.Label>
            <Muted text="Name" />
          </Field.Label>
          <Field.Control
            value={name()}
            onInput={(event) => setName(event.value ?? "")}
            style={{ ...inputStyle(), width: 260 }}
          />
        </Field.Root>
        <Field.Root style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          <Field.Label>
            <Muted text="Organisation" />
          </Field.Label>
          <Field.Control
            value={org()}
            onInput={(event) => setOrg(event.value ?? "")}
            style={{ ...inputStyle(), width: 260 }}
          />
        </Field.Root>
      </Fieldset.Root>
      <Note text={`disabled ${saving()} · name "${name()}" · org "${org()}"`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Form
 *
 * `@quickgui/solid` binds no `Form` compound of its own: a form is a Fieldset of Field roots whose
 * `validationMode` defaults to `"onSubmit"`, plus the submit edge an `Input` reports.
 * -------------------------------------------------------------------------------------------- */

function FormDemo() {
  const [email, setEmail] = createSignal("");
  const [plan, setPlan] = createSignal("pro");
  const [terms, setTerms] = createSignal(false);
  const [submitted, setSubmitted] = createSignal("nothing submitted yet");
  const [attempts, setAttempts] = createSignal(0);
  const [showErrors, setShowErrors] = createSignal(false);

  const emailValid = () => email().includes("@");
  const formValid = () => emailValid() && terms();

  const submit = () => {
    setAttempts(attempts() + 1);
    if (!formValid()) {
      setShowErrors(true);
      setSubmitted("rejected: fix the fields below");
      return;
    }
    setShowErrors(false);
    setSubmitted(`accepted: ${email()} on the ${plan()} plan`);
  };

  return (
    <Panel
      title="Form"
      hint="validationMode defaults to onSubmit, so nothing is marked invalid until the form is really submitted. Return inside the field submits it too."
    >
      <Fieldset.Root
        style={{
          display: "flex",
          flexDirection: "column",
          gap: 12,
          padding: 14,
          borderRadius: 10,
          borderWidth: 1,
          borderColor: p().border,
          backgroundColor: p().panelAlt,
        }}
      >
        <Fieldset.Legend>
          <Text style={{ fontSize: 12, fontWeight: 700, color: p().ink }}>
            Sign up
          </Text>
        </Fieldset.Legend>

        <Field.Root
          required
          invalid={showErrors() && !emailValid()}
          touched={attempts() > 0}
          filled={email().length > 0}
          validationMessage="An address is required"
          style={{ display: "flex", flexDirection: "column", gap: 4 }}
        >
          <Field.Label>
            <Muted text="Email" />
          </Field.Label>
          <Field.Control
            value={email()}
            placeholder="you@example.com"
            onInput={(event) => setEmail(event.value ?? "")}
            onSubmit={() => submit()}
            style={{ ...inputStyle(), width: 280 }}
          />
          <Show when={showErrors() && !emailValid()}>
            <Text style={{ fontSize: 12, color: p().danger }}>
              An address containing @ is required
            </Text>
          </Show>
        </Field.Root>

        <Field.Root style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          <Field.Label passive>
            <Muted text="Plan" />
          </Field.Label>
          <RadioGroup.Root
            value={plan()}
            onValueChange={setPlan}
            required
            style={{ display: "flex", flexDirection: "row", gap: 10 }}
          >
            <For each={["hobby", "pro", "team"]}>
              {(value) => (
                <Radio.Root
                  value={value}
                  style={{
                    display: "flex",
                    flexDirection: "row",
                    alignItems: "center",
                    gap: 6,
                    height: 26,
                    paddingLeft: 8,
                    paddingRight: 10,
                    borderRadius: 8,
                    hoverBackgroundColor: p().controlHover,
                    focusOutline: `2px solid ${p().accent}`,
                  }}
                >
                  <Radio.Indicator
                    style={{
                      width: 14,
                      height: 14,
                      borderRadius: 7,
                      borderWidth: plan() === value ? 4 : 1,
                      borderColor: plan() === value ? p().accent : p().border,
                      backgroundColor: p().control,
                    }}
                  />
                  <Label text={value} />
                </Radio.Root>
              )}
            </For>
          </RadioGroup.Root>
        </Field.Root>

        <Checkbox.Root
          checked={terms()}
          onCheckedChange={setTerms}
          style={{
            display: "flex",
            flexDirection: "row",
            alignItems: "center",
            gap: 8,
            height: 26,
          }}
        >
          <Checkbox.Indicator
            style={{
              width: 16,
              height: 16,
              borderRadius: 5,
              borderWidth: 1,
              borderColor: terms() ? p().accent : p().border,
              backgroundColor: terms() ? p().accent : p().control,
            }}
          />
          <Label text="I accept the terms" />
        </Checkbox.Root>
        <Show when={showErrors() && !terms()}>
          <Text style={{ fontSize: 12, color: p().danger }}>
            The terms must be accepted
          </Text>
        </Show>

        <Row>
          <Btn label="Submit" primary onClick={submit} />
          <Btn
            label="Reset"
            onClick={() => {
              setEmail("");
              setPlan("pro");
              setTerms(false);
              setShowErrors(false);
              setAttempts(0);
              setSubmitted("nothing submitted yet");
            }}
          />
        </Row>
      </Fieldset.Root>
      <Note text={`attempts ${attempts()} · ${submitted()}`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Input
 * -------------------------------------------------------------------------------------------- */

function InputDemo() {
  const [text, setText] = createSignal("");
  const [secret, setSecret] = createSignal("");
  const [notes, setNotes] = createSignal("Two\nlines");
  const [submits, setSubmits] = createSignal(0);

  return (
    <Panel
      title="Input"
      hint="Controlled native editors. Update value from event.value in onInput; Return on a single-line input reports onSubmit."
    >
      <Col>
        <Muted text="text · Return submits" />
        <Input
          value={text()}
          placeholder="Type and press Return"
          onInput={(event) => setText(event.value ?? "")}
          onSubmit={() => setSubmits(submits() + 1)}
          style={{ ...inputStyle(), width: 300 }}
        />
        <Muted text="password" />
        <Input
          type="password"
          value={secret()}
          placeholder="Secret"
          onInput={(event) => setSecret(event.value ?? "")}
          style={{ ...inputStyle(), width: 300 }}
        />
        <Muted text="multiline" />
        <Input
          multiline
          value={notes()}
          onInput={(event) => setNotes(event.value ?? "")}
          style={{ ...inputStyle(), width: 300, height: 72, paddingTop: 6 }}
        />
      </Col>
      <Note
        text={`text "${text()}" · password ${secret().length} chars · notes ${notes().split("\n").length} lines · submits ${submits()}`}
      />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Menu
 * -------------------------------------------------------------------------------------------- */

function MenuRowLabel(props: { label: string }) {
  const state = useMenuItemState();
  return (
    <Text
      style={{
        fontSize: 12,
        color: state().highlighted ? p().onAccent : p().ink,
      }}
    >
      {props.label}
    </Text>
  );
}

function MenuRow(props: {
  value: string;
  label: string;
  onActivate: (value: string) => void;
}) {
  return (
    <Menu.Item
      value={props.value}
      label={props.label}
      onClick={() => props.onActivate(props.value)}
      style={{
        ...menuRowStyle(),
        hoverBackgroundColor: p().selection,
      }}
    >
      <MenuRowLabel label={props.label} />
    </Menu.Item>
  );
}

function MenuDemo() {
  const [open, setOpen] = createSignal(false);
  const [wrap, setWrap] = createSignal(false);
  const [density, setDensity] = createSignal("cozy");
  const [activated, setActivated] = createSignal("nothing yet");

  return (
    <Panel
      title="Menu"
      hint="The Base UI menu compound: application-styled rows whose identity, roving highlight, toggle policy, radio exclusivity, and closing are all the core's."
    >
      <Menu.Root
        open={open()}
        onOpenChange={setOpen}
        side="bottom"
        align="start"
        sideOffset={6}
      >
        <Menu.Trigger style={controlStyle()}>
          <Text style={{ fontSize: 12, color: p().ink }}>Edit ▾</Text>
        </Menu.Trigger>
        <Menu.Positioner>
          <Menu.Popup style={{ ...popupStyle(), width: 250, padding: 5, gap: 2 }}>
            <Menu.GroupLabel
              value="clipboard"
              label="Clipboard"
              style={{ paddingLeft: 10, height: 20, justifyContent: "center" }}
            >
              <Text style={{ fontSize: 11, color: p().faint }}>Clipboard</Text>
            </Menu.GroupLabel>
            <MenuRow value="copy" label="Copy" onActivate={setActivated} />
            <MenuRow value="cut" label="Cut" onActivate={setActivated} />
            <Menu.LinkItem
              value="docs"
              label="Documentation"
              href="https://quickgui.dev"
              style={menuRowStyle()}
              onNavigate={(href) => setActivated(`link ${href}`)}
            >
              <MenuRowLabel label="Documentation" />
            </Menu.LinkItem>
            <Menu.Separator
              style={{
                height: 1,
                backgroundColor: p().border,
                marginTop: 4,
                marginBottom: 4,
              }}
            />
            <Menu.CheckboxItem
              value="wrap"
              label="Wrap lines"
              checked={wrap()}
              onCheckedChange={setWrap}
              style={menuRowStyle()}
            >
              <MenuRowLabel label="Wrap lines" />
              <Menu.CheckboxItemIndicator>
                <Text style={{ fontSize: 12, color: p().accent }}>✓</Text>
              </Menu.CheckboxItemIndicator>
            </Menu.CheckboxItem>
            <Menu.RadioGroup
              name="density"
              value={density()}
              onValueChange={setDensity}
            >
              <For each={["compact", "cozy"]}>
                {(value) => (
                  <Menu.RadioItem value={value} label={value} style={menuRowStyle()}>
                    <MenuRowLabel label={value} />
                    <Menu.RadioItemIndicator>
                      <Text style={{ fontSize: 12, color: p().accent }}>●</Text>
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
                style={menuRowStyle()}
              >
                <MenuRowLabel label="Open recent" />
                <Text style={{ fontSize: 12, color: p().muted }}>›</Text>
              </Menu.SubmenuTrigger>
              <Menu.Positioner side="right" align="start" sideOffset={4}>
                <Menu.Popup style={{ ...popupStyle(), width: 200, padding: 5 }}>
                  <MenuRow value="notes" label="notes.md" onActivate={setActivated} />
                  <MenuRow value="readme" label="README.md" onActivate={setActivated} />
                </Menu.Popup>
              </Menu.Positioner>
            </Menu.SubmenuRoot>
          </Menu.Popup>
        </Menu.Positioner>
      </Menu.Root>
      <Note
        text={`open ${open()} · activated ${activated()} · wrap ${wrap()} · density ${density()}`}
      />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Menubar
 * -------------------------------------------------------------------------------------------- */

function MenubarDemo() {
  const menus = [
    {
      label: "File",
      items: [
        { id: "new", label: "New window", shortcut: "⌘N" },
        { id: "open", label: "Open…", shortcut: "⌘O" },
        { type: "separator" as const },
        { id: "close", label: "Close", shortcut: "⌘W" },
      ],
    },
    {
      label: "Edit",
      items: [
        { id: "undo", label: "Undo", shortcut: "⌘Z" },
        { id: "redo", label: "Redo", shortcut: "⇧⌘Z" },
        { type: "separator" as const },
        { id: "paste", label: "Paste", shortcut: "⌘V" },
      ],
    },
    {
      label: "View",
      items: [
        { id: "zoom-in", label: "Zoom in", shortcut: "⌘+" },
        { id: "zoom-out", label: "Zoom out", shortcut: "⌘−" },
      ],
    },
  ];
  const [openMenu, setOpenMenu] = createSignal<number | null>(null);
  const [active, setActive] = createSignal(0);
  const [command, setCommand] = createSignal("nothing yet");

  return (
    <Panel
      title="Menubar"
      hint="One roving Tab stop across the bar. Arrow keys keep switching menus while one is open; Escape closes without leaving the bar."
    >
      <Menubar.Root
        scope="bar"
        count={menus.length}
        open={openMenu()}
        onOpenChange={(index) => setOpenMenu(index ?? null)}
        onActiveChange={setActive}
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
        <For each={menus}>
          {(menu, index) => (
            <PopoverMenu.Root
              items={menu.items}
              appearance={menuAppearance()}
              open={openMenu() === index()}
              onOpenChange={(next) => setOpenMenu(next ? index() : null)}
              onSelect={(details) => setCommand(details.id)}
            >
              <PopoverMenu.Trigger style={{ display: "flex" }}>
                <Menubar.Item
                  scope="bar"
                  itemIndex={index()}
                  style={{
                    ...controlStyle(),
                    height: 26,
                    backgroundColor:
                      openMenu() === index() ? p().selection : "transparent",
                    borderColor: "transparent",
                  }}
                >
                  <Text style={{ fontSize: 12, color: p().ink }}>{menu.label}</Text>
                </Menubar.Item>
              </PopoverMenu.Trigger>
              <PopoverMenu.Popup
                style={{ backgroundColor: p().popup, borderRadius: 10, width: 220 }}
              />
            </PopoverMenu.Root>
          )}
        </For>
      </Menubar.Root>
      <Note
        text={`open ${openMenu() ?? "none"} · tab stop ${active()} · command ${command()}`}
      />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Meter
 * -------------------------------------------------------------------------------------------- */

function GaugeReadout() {
  const gauge = useGaugeState();
  return (
    <Note
      text={`status ${gauge().status} · display ${gauge().displayValue ?? "—"} · completion ${gauge().completion === null ? "—" : gauge().completion!.toFixed(2)}`}
    />
  );
}

function MeterDemo() {
  const [level, setLevel] = createSignal<readonly number[]>([62]);
  const value = () => level()[0] ?? 0;
  const zone = () => (value() < 25 ? p().danger : value() > 80 ? "#c88a00" : p().accent);

  return (
    <Panel
      title="Meter"
      hint="A level inside a known range, not task progress. low/high/optimum let the application colour the gauge without the framework inventing thresholds."
    >
      <Meter.Root
        scope="meter-disk"
        value={value()}
        min={0}
        max={100}
        low={25}
        high={80}
        optimum={50}
        format="percent"
        style={{ display: "flex", flexDirection: "column", gap: 6 }}
      >
        <Row>
          <Meter.Label scope="meter-disk">
            <Muted text="Disk used" />
          </Meter.Label>
          <Meter.Value scope="meter-disk">
            <Text style={{ fontSize: 12, color: p().ink }}>{`${value()}%`}</Text>
          </Meter.Value>
        </Row>
        <Meter.Track
          scope="meter-disk"
          style={{
            display: "flex",
            flexDirection: "row",
            width: GAUGE_WIDTH,
            height: 8,
            borderRadius: 4,
            backgroundColor: p().track,
            overflow: "hidden",
          }}
        >
          <Meter.Indicator
            scope="meter-disk"
            style={{
              height: 8,
              borderRadius: 4,
              backgroundColor: zone(),
              width: `${value()}%`,
            }}
          />
        </Meter.Track>
        <GaugeReadout />
      </Meter.Root>

      <Slider.Root
        scope="meter-slider"
        value={level()}
        min={0}
        max={100}
        step={1}
        onValueChange={setLevel}
        style={{
          display: "flex",
          flexDirection: "row",
          alignItems: "center",
          width: GAUGE_WIDTH,
          height: 22,
        }}
      >
        <Slider.Control
          scope="meter-slider"
          style={{
            position: "relative",
            display: "flex",
            flexDirection: "row",
            alignItems: "center",
            width: GAUGE_WIDTH,
            height: 22,
          }}
        >
          <Slider.Track
            scope="meter-slider"
            style={{
              display: "flex",
              flexDirection: "row",
              width: GAUGE_WIDTH,
              height: 5,
              borderRadius: 3,
              backgroundColor: p().track,
            }}
          >
            <Slider.Indicator
              scope="meter-slider"
              style={{
                height: 5,
                borderRadius: 3,
                backgroundColor: p().accent,
                width: `${value()}%`,
              }}
            />
          </Slider.Track>
          <Slider.Thumb
            scope="meter-slider"
            index={0}
            style={{
              position: "absolute",
              left: Math.round((value() / 100) * (GAUGE_WIDTH - 14)),
              top: 4,
              width: 14,
              height: 14,
              borderRadius: 7,
              backgroundColor: p().accent,
              focusOutline: `2px solid ${p().accent}`,
              outlineOffset: 2,
            }}
          />
        </Slider.Control>
      </Slider.Root>
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Navigation menu
 * -------------------------------------------------------------------------------------------- */

function NavigationMenuDemo() {
  const items = ["products", "solutions", "support"];
  const [value, setValue] = createSignal("none");
  const [direction, setDirection] = createSignal("none");

  return (
    <Panel
      title="Navigation menu"
      hint="A Navigation landmark with one Tab stop. Hovering opens after the exact delay and switches immediately once a panel is open; the activation direction is reported so the panel can slide the right way."
    >
      <NavigationMenu.Root
        delay={80}
        closeDelay={80}
        onValueChange={(next) => setValue(next ?? "none")}
        onActivationDirectionChange={setDirection}
        items={[
          { value: "products" },
          { value: "solutions" },
          { value: "support", disabled: true },
        ]}
      >
        <NavigationMenu.List
          style={{ display: "flex", flexDirection: "row", gap: 6 }}
        >
          <For each={items}>
            {(item) => (
              <NavigationMenu.Item value={item}>
                <NavigationMenu.Trigger
                  style={{
                    ...controlStyle(),
                    opacity: item === "support" ? 0.5 : 1,
                    backgroundColor:
                      value() === item ? p().selection : p().control,
                  }}
                >
                  <Text style={{ fontSize: 12, color: p().ink }}>{item}</Text>
                </NavigationMenu.Trigger>
                <NavigationMenu.Positioner>
                  <NavigationMenu.Popup style={{ ...popupStyle(), width: 230 }}>
                    <NavigationMenu.Viewport>
                      <NavigationMenu.Content
                        style={{ display: "flex", flexDirection: "column", gap: 4 }}
                      >
                        <NavigationMenu.Link value={`${item}-overview`} active>
                          <Text style={{ fontSize: 12, color: p().accent }}>
                            {`${item} overview`}
                          </Text>
                        </NavigationMenu.Link>
                        <NavigationMenu.Link value={`${item}-pricing`}>
                          <Text style={{ fontSize: 12, color: p().ink }}>
                            {`${item} pricing`}
                          </Text>
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
      <Note text={`open panel ${value()} · activation direction ${direction()}`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Number field
 * -------------------------------------------------------------------------------------------- */

function NumberFieldScrubCursor() {
  const field = useNumberFieldState();
  return (
    <NumberField.ScrubAreaCursor
      scope="nf-qty"
      style={{
        width: 12,
        height: 12,
        borderRadius: 6,
        backgroundColor: field().scrubbing ? p().accent : p().border,
      }}
    />
  );
}

function NumberFieldStateLine() {
  const field = useNumberFieldState();
  return (
    <Note
      text={`scrubbing ${field().scrubbing} · readOnly ${field().readOnly} · required ${field().required}`}
    />
  );
}

function NumberFieldDemo() {
  const [quantity, setQuantity] = createSignal(8);
  const [valid, setValid] = createSignal(true);
  const [committed, setCommitted] = createSignal<number | undefined>();

  return (
    <Panel
      title="Number field"
      hint="The core parses, clamps, snaps, and formats. Alt is the small step, Shift the large one, the wheel scrubs while focused, and Return is the commit boundary."
    >
      <NumberField.Root
        scope="nf-qty"
        value={quantity()}
        min={0}
        max={99}
        step={1}
        smallStep={0.5}
        largeStep={10}
        precision={1}
        snapOnStep
        required
        scrubDirection="horizontal"
        onValueChange={(next, isValid) => {
          setQuantity(next ?? 0);
          setValid(isValid);
        }}
        onValueCommitted={(next) => setCommitted(next)}
      >
        <NumberField.Group scope="nf-qty">
          <Row>
            <NumberField.Decrement
              scope="nf-qty"
              style={{ ...controlStyle(), width: 32, paddingLeft: 0, paddingRight: 0 }}
            >
              <Text style={{ fontSize: 14, color: p().ink }}>−</Text>
            </NumberField.Decrement>
            <NumberField.Input
              scope="nf-qty"
              style={{ ...inputStyle(), width: 84, textAlign: "center" }}
            />
            <NumberField.Increment
              scope="nf-qty"
              style={{ ...controlStyle(), width: 32, paddingLeft: 0, paddingRight: 0 }}
            >
              <Text style={{ fontSize: 14, color: p().ink }}>+</Text>
            </NumberField.Increment>
            <NumberField.ScrubArea
              scope="nf-qty"
              style={{
                width: 44,
                height: 30,
                borderRadius: 8,
                borderColor: p().border,
                borderWidth: 1,
                alignItems: "center",
                justifyContent: "center",
                cursor: "ew-resize",
              }}
            >
              <NumberFieldScrubCursor />
            </NumberField.ScrubArea>
          </Row>
        </NumberField.Group>
        <NumberFieldStateLine />
      </NumberField.Root>
      <Note
        text={`value ${quantity()} · valid ${valid()} · committed ${committed() === undefined ? "—" : committed()}`}
      />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * OTP field
 * -------------------------------------------------------------------------------------------- */

function OtpFieldDemo() {
  const [code, setCode] = createSignal("");
  const [completed, setCompleted] = createSignal("not yet");

  return (
    <Panel
      title="OTP field"
      hint="Six one-character slots. Typing advances, a paste distributes, Backspace clears then walks back, and the completion edge is reported once."
    >
      <OtpField.Root
        length={6}
        value={code()}
        validationType="numeric"
        required
        onValueChange={setCode}
        onComplete={setCompleted}
        style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 6 }}
      >
        <For each={[0, 1, 2, 3, 4, 5]}>
          {(index) => (
            <>
              <OtpField.Input
                index={index}
                style={{
                  ...inputStyle(),
                  width: 38,
                  height: 44,
                  paddingLeft: 0,
                  paddingRight: 0,
                  fontSize: 17,
                  textAlign: "center",
                }}
              />
              <Show when={index === 2}>
                <OtpField.Separator index={index}>
                  <Text style={{ fontSize: 15, color: p().faint }}>–</Text>
                </OtpField.Separator>
              </Show>
            </>
          )}
        </For>
      </OtpField.Root>
      <Note text={`code "${code()}" · completed ${completed()}`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Popover
 * -------------------------------------------------------------------------------------------- */

function ResolvedPlacement() {
  const placement = usePopoverPlacement();
  return (
    <Note
      text={`resolved ${placement().side}/${placement().align} · anchor hidden ${placement().anchorHidden ?? false}`}
    />
  );
}

function PopoverDemo() {
  const [open, setOpen] = createSignal(false);
  const [hover, setHover] = createSignal(false);

  return (
    <Panel
      title="Popover"
      hint="The declared side is only a preference; the panel prints the placement the retained tree really resolved to."
    >
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
              ...controlStyle(),
              backgroundColor: p().accent,
              borderColor: p().accent,
              hoverBackgroundColor: p().accentHover,
            }}
          >
            <Text style={{ fontSize: 12, color: p().onAccent }}>Account</Text>
          </Popover.Trigger>
          <ResolvedPlacement />
        </Row>
        <Popover.Positioner>
          <Popover.Popup style={{ ...popupStyle(), width: 240 }}>
            <Popover.Arrow
              style={{ width: 10, height: 10, backgroundColor: p().popup }}
            />
            <Popover.Title>
              <Text style={{ fontSize: 13, fontWeight: 700, color: p().ink }}>
                Signed in
              </Text>
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

      <Popover.Root
        openOnHover
        delay={200}
        closeDelay={120}
        side="right"
        align="center"
        sideOffset={8}
        onOpenChange={setHover}
      >
        <Popover.Trigger style={controlStyle()}>
          <Text style={{ fontSize: 12, color: p().ink }}>Hover to open</Text>
        </Popover.Trigger>
        <Popover.Positioner>
          <Popover.Popup style={{ ...popupStyle(), width: 200 }}>
            <Muted text="Opened on the core's exact hover deadline." />
          </Popover.Popup>
        </Popover.Positioner>
      </Popover.Root>
      <Note text={`click popover ${open()} · hover popover ${hover()}`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Preview card
 * -------------------------------------------------------------------------------------------- */

function PreviewCardDemo() {
  const [open, setOpen] = createSignal(false);

  return (
    <Panel
      title="Preview card"
      hint="The trigger projects the Link role. Resting the pointer opens on the core's exact delay; focus opens it immediately, because a keyboard user cannot rest a pointer."
    >
      <Row>
        <Muted text="Written by" />
        <PreviewCard.Root
          open={open()}
          onOpenChange={setOpen}
          placement="bottom-start"
          gap={8}
        >
          <PreviewCard.Trigger delay={350} closeDelay={200} style={{ padding: 2 }}>
            <Text style={{ fontSize: 12, color: p().accent, textDecoration: "underline" }}>
              @ada
            </Text>
          </PreviewCard.Trigger>
          <PreviewCard.Positioner>
            <PreviewCard.Popup style={{ ...popupStyle(), width: 240 }}>
              <Text style={{ fontSize: 13, fontWeight: 700, color: p().ink }}>
                Ada Lovelace
              </Text>
              <Muted text="Wrote the first algorithm intended for a machine." />
            </PreviewCard.Popup>
          </PreviewCard.Positioner>
        </PreviewCard.Root>
      </Row>
      <Note text={`open ${open()}`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Progress
 * -------------------------------------------------------------------------------------------- */

function ProgressDemo() {
  const total = 12;
  const [done, setDone] = createSignal(3);
  const [indeterminate, setIndeterminate] = createSignal(false);

  return (
    <Panel
      title="Progress"
      hint="The core derives the status — progressing, complete, or indeterminate — and formats the value. Indeterminate motion stays application-owned so no framework animation keeps the window awake."
    >
      <Progress.Root
        scope="upload"
        {...(indeterminate() ? {} : { value: done() })}
        max={total}
        indeterminate={indeterminate()}
        format="fraction"
        valueText={`${done()} of ${total} files`}
        style={{ display: "flex", flexDirection: "column", gap: 6 }}
      >
        <Row>
          <Progress.Label scope="upload">
            <Muted text="Uploading" />
          </Progress.Label>
          <Progress.Value scope="upload">
            <Text style={{ fontSize: 12, color: p().ink }}>
              {indeterminate() ? "…" : `${done()} / ${total}`}
            </Text>
          </Progress.Value>
        </Row>
        <Progress.Track
          scope="upload"
          style={{
            display: "flex",
            flexDirection: "row",
            width: GAUGE_WIDTH,
            height: 8,
            borderRadius: 4,
            backgroundColor: p().track,
            overflow: "hidden",
          }}
        >
          <Progress.Indicator
            scope="upload"
            style={{
              height: 8,
              borderRadius: 4,
              backgroundColor: p().accent,
              width: indeterminate()
                ? "35%"
                : `${Math.round((done() / total) * 100)}%`,
            }}
          />
        </Progress.Track>
        <GaugeReadout />
      </Progress.Root>
      <Row>
        <Btn label="−1" onClick={() => setDone(Math.max(0, done() - 1))} />
        <Btn label="+1" onClick={() => setDone(Math.min(total, done() + 1))} />
        <Btn label="Complete" onClick={() => setDone(total)} />
        <Btn
          label={indeterminate() ? "Determinate" : "Indeterminate"}
          onClick={() => setIndeterminate(!indeterminate())}
        />
      </Row>
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Radio
 * -------------------------------------------------------------------------------------------- */

function RadioDemo() {
  const [theme, setTheme] = createSignal("system");
  const [locked, setLocked] = createSignal("b");

  const row = () =>
    ({
      display: "flex",
      flexDirection: "row",
      alignItems: "center",
      gap: 8,
      height: 28,
      paddingLeft: 8,
      paddingRight: 12,
      borderRadius: 8,
      hoverBackgroundColor: p().controlHover,
      focusOutline: `2px solid ${p().accent}`,
    }) as const;

  const dot = (on: boolean) =>
    ({
      width: 15,
      height: 15,
      borderRadius: 8,
      borderWidth: on ? 4 : 1,
      borderColor: on ? p().accent : p().border,
      backgroundColor: p().control,
    }) as const;

  return (
    <Panel
      title="Radio"
      hint="Exactly one selection per group. Arrow keys move and select, the group keeps a single Tab stop, and readOnly refuses changes while staying focusable."
    >
      <RadioGroup.Root
        value={theme()}
        onValueChange={setTheme}
        required
        style={{ display: "flex", flexDirection: "column", gap: 2 }}
      >
        <For each={["light", "dark", "system"]}>
          {(value) => (
            <Radio.Root value={value} style={row()}>
              <Radio.Indicator style={dot(theme() === value)} />
              <Label text={value} />
            </Radio.Root>
          )}
        </For>
      </RadioGroup.Root>

      <Separator.Root style={{ height: 1, backgroundColor: p().border }} />

      <RadioGroup.Root
        value={locked()}
        readOnly
        style={{ display: "flex", flexDirection: "row", gap: 8 }}
      >
        <For each={["a", "b", "c"]}>
          {(value) => (
            <Radio.Root value={value} style={row()}>
              <Radio.Indicator style={dot(locked() === value)} />
              <Muted text={`read-only ${value}`} />
            </Radio.Root>
          )}
        </For>
      </RadioGroup.Root>
      <Note text={`theme ${theme()} · read-only group ${locked()}`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Scroll area
 * -------------------------------------------------------------------------------------------- */

/**
 * The thumb, styled from the state the core reports for it.
 *
 * The native side owns the thumb's position and length — both are computed from the measured
 * geometry — so the application declares only its width and its paint.
 */
function ScrollAreaThumbBody() {
  const state = useScrollAreaState();
  return (
    <ScrollArea.Thumb
      orientation="vertical"
      style={{
        width: 8,
        borderRadius: 4,
        backgroundColor: state().scrolling ? p().accent : p().border,
      }}
    />
  );
}

function ScrollAreaDemo() {
  const rowHeight = 22;
  const rows = Array.from({ length: 40 }, (_, index) => `log line ${index + 1}`);
  const viewport = { width: 320, height: 160 };
  const content = { width: 320, height: rows.length * rowHeight };
  const [state, setState] = createSignal<ScrollAreaState | undefined>();
  const offsetY = () => state()?.offset.y ?? 0;

  return (
    <Panel
      title="Scroll area"
      hint="A declared model: the extents are declared here (the core also measures them from painted bounds), the core clamps the offset, derives every overflow flag, and positions and sizes the thumb itself. The offset is applied to the content as a paint-only transform."
    >
      <ScrollArea.Root
        viewportSize={viewport}
        contentSize={content}
        overflowEdgeThreshold={2}
        onScrollStateChange={setState}
        style={{
          display: "flex",
          flexDirection: "row",
          gap: 4,
          height: viewport.height,
        }}
      >
        <ScrollArea.Viewport
          style={{
            width: viewport.width,
            height: viewport.height,
            backgroundColor: p().panelAlt,
            borderRadius: 10,
            borderWidth: 1,
            borderColor: p().border,
            overflow: "hidden",
          }}
        >
          <ScrollArea.Content
            style={{
              display: "flex",
              flexDirection: "column",
              paddingLeft: 10,
              transform: `translateY(${-offsetY()}px)`,
            }}
          >
            <For each={rows}>
              {(row) => (
                <Text
                  style={{
                    fontSize: 12,
                    color: p().muted,
                    height: rowHeight,
                    lineHeight: rowHeight,
                  }}
                >
                  {row}
                </Text>
              )}
            </For>
          </ScrollArea.Content>
        </ScrollArea.Viewport>
        <ScrollArea.Scrollbar
          orientation="vertical"
          style={{
            width: 8,
            height: viewport.height,
            backgroundColor: p().track,
            borderRadius: 4,
          }}
        >
          <ScrollAreaThumbBody />
        </ScrollArea.Scrollbar>
      </ScrollArea.Root>
      <Note
        text={`offset ${Math.round(offsetY())} · scrolling ${state()?.scrolling ?? false} · hasOverflowY ${state()?.hasOverflowY ?? false} · overflowYStart ${state()?.overflowYStart ?? false} · overflowYEnd ${state()?.overflowYEnd ?? false}`}
      />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Select
 * -------------------------------------------------------------------------------------------- */

function SelectValueText(props: { text: () => string; placeholder: string }) {
  const state = useSelectState();
  return (
    <Text
      style={{ fontSize: 12, color: state().placeholder ? p().muted : p().ink }}
    >
      {state().placeholder ? props.placeholder : props.text()}
    </Text>
  );
}

function SelectStateLine() {
  const state = useSelectState();
  return (
    <Note
      text={`popup ${state().popupOpen} · side ${state().popupSide} · filled ${state().filled} · touched ${state().touched}`}
    />
  );
}

function SelectDemo() {
  const [theme, setTheme] = createSignal("system");
  const [sizes, setSizes] = createSignal<readonly string[]>(["m"]);

  return (
    <Panel
      title="Select"
      hint="A native popover window the core renders itself from the declared appearance. The multiple select declares its options in the items map form and keeps the declared order."
    >
      <Select.Root
        scope="sel-theme"
        ariaLabel="Theme"
        value={theme()}
        items={[
          { value: "light", label: "Light" },
          { value: "dark", label: "Dark", detail: "⌘D" },
          { value: "system", label: "Match system" },
        ]}
        appearance={pickerAppearance()}
        onValueChange={(next) => setTheme(next ?? "")}
        style={{ ...controlStyle(), width: 210, justifyContent: "space-between" }}
      >
        <Select.Value>
          <Text style={{ fontSize: 12, color: p().ink }}>{theme()}</Text>
        </Select.Value>
        <Select.Icon>
          <Text style={{ fontSize: 11, color: p().muted }}>▾</Text>
        </Select.Icon>
      </Select.Root>

      <Select.Root
        scope="sel-sizes"
        ariaLabel="Sizes"
        multiple
        values={sizes()}
        onValuesChange={setSizes}
        alignItemWithTrigger
        appearance={pickerAppearance()}
        items={{ s: "Small", m: "Medium", l: "Large", xl: "Extra large" }}
        style={{ ...controlStyle(), width: 210, justifyContent: "space-between" }}
      >
        <Select.Value>
          <SelectValueText text={() => sizes().join(", ")} placeholder="Pick sizes" />
        </Select.Value>
        <Select.Icon>
          <Text style={{ fontSize: 11, color: p().muted }}>▾</Text>
        </Select.Icon>
        <Select.Positioner side="bottom" align="start" sideOffset={6}>
          <Select.ScrollUpArrow />
          <Select.ScrollDownArrow />
        </Select.Positioner>
        <SelectStateLine />
      </Select.Root>
      <Note text={`theme ${theme()} · sizes [${sizes().join(", ")}]`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Separator
 * -------------------------------------------------------------------------------------------- */

function SeparatorDemo() {
  return (
    <Panel
      title="Separator"
      hint="Orientation and nothing else: no thickness, no colour, no inset. Never focusable, and never part of a neighbouring accessible name."
    >
      <Col gap={10}>
        <Label text="Above the rule" />
        <Separator.Root
          orientation="horizontal"
          style={{ height: 1, backgroundColor: p().border }}
        />
        <Label text="Below the rule" />
      </Col>
      <Row>
        <Muted text="Cut" />
        <Separator.Root
          orientation="vertical"
          style={{ width: 1, height: 18, backgroundColor: p().border }}
        />
        <Muted text="Copy" />
        <Separator.Root
          orientation="vertical"
          style={{ width: 1, height: 18, backgroundColor: p().border }}
        />
        <Muted text="Paste" />
      </Row>
      <Note text="an adjustable divider is Splitter, which is focusable and carries a value" />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Slider
 * -------------------------------------------------------------------------------------------- */

function SliderReadout() {
  const slider = useSliderState();
  return (
    <Text
      style={{
        fontSize: 12,
        color: slider().dragging ? p().accent : p().muted,
        fontWeight: slider().dragging ? 700 : 400,
      }}
    >
      {slider().displayValue ?? "—"}
    </Text>
  );
}

function SliderDemo() {
  const [volume, setVolume] = createSignal<readonly number[]>([40]);
  const [committed, setCommitted] = createSignal<number | undefined>();
  const [range, setRange] = createSignal<readonly number[]>([20, 70]);

  // The look of an AppKit slider: a 4px track with the accent fill to the left of a 20px white
  // knob that carries a hairline and a soft shadow. Pressing anywhere on the control jumps the
  // nearest knob there and continues as a drag, which is the core's own pointer policy.
  const KNOB = 20;
  const control = () =>
    ({
      position: "relative",
      display: "flex",
      flexDirection: "row",
      alignItems: "center",
      width: GAUGE_WIDTH,
      height: 24,
    }) as const;
  // The core owns the value; the application owns where the knob is painted.
  const thumbLeft = (value: number) => Math.round((value / 100) * (GAUGE_WIDTH - KNOB));
  const track = () =>
    ({
      display: "flex",
      flexDirection: "row",
      width: GAUGE_WIDTH,
      height: 4,
      borderRadius: 2,
      backgroundColor: p().track,
      borderWidth: 1,
      borderColor: p().border,
    }) as const;
  const fill = () =>
    ({
      height: 4,
      borderRadius: 2,
      marginTop: -1,
      marginLeft: -1,
      backgroundColor: p().accent,
    }) as const;
  const thumb = (value: number) =>
    ({
      position: "absolute",
      left: thumbLeft(value),
      top: 2,
      width: KNOB,
      height: KNOB,
      borderRadius: KNOB / 2,
      backgroundColor: p().panel,
      borderWidth: 1,
      borderColor: p().border,
      boxShadow: "0 1px 2px #0000003d",
      activeBackgroundColor: p().controlHover,
      focusOutline: `2px solid ${p().accent}`,
      outlineOffset: 2,
    }) as const;

  return (
    <Panel
      title="Slider"
      hint="Clamping, step snapping, thumb ordering, and the captured pointer arithmetic are all the core's. onValueCommitted fires on the frame the gesture released."
    >
      <Slider.Root
        scope="sl-volume"
        value={volume()}
        min={0}
        max={100}
        step={5}
        largeStep={25}
        format="percent"
        onValueChange={setVolume}
        onValueCommitted={(values) => setCommitted(values[0])}
        style={{ display: "flex", flexDirection: "column", gap: 6, width: GAUGE_WIDTH }}
      >
        <Row>
          <Slider.Label scope="sl-volume">
            <Muted text="Volume" />
          </Slider.Label>
          <Slider.Value scope="sl-volume">
            <SliderReadout />
          </Slider.Value>
        </Row>
        <Slider.Control scope="sl-volume" style={control()}>
          <Slider.Track scope="sl-volume" style={track()}>
            <Slider.Indicator
              scope="sl-volume"
              style={{ ...fill(), width: `${volume()[0] ?? 0}%` }}
            />
          </Slider.Track>
          <Slider.Thumb scope="sl-volume" index={0} style={thumb(volume()[0] ?? 0)} />
        </Slider.Control>
      </Slider.Root>
      <Note
        text={`volume ${volume()[0] ?? 0} · committed ${committed() === undefined ? "—" : committed()}`}
      />

      <Slider.Root
        scope="sl-range"
        value={range()}
        min={0}
        max={100}
        step={1}
        minStepsBetweenValues={5}
        onValueChange={setRange}
        style={{ display: "flex", flexDirection: "column", gap: 6, width: GAUGE_WIDTH }}
      >
        <Slider.Label scope="sl-range">
          <Muted text="Range · the core holds five steps open between the thumbs" />
        </Slider.Label>
        <Slider.Control scope="sl-range" style={control()}>
          <Slider.Track scope="sl-range" style={track()}>
            <Slider.Range
              scope="sl-range"
              style={{
                ...fill(),
                marginLeft: Math.round(((range()[0] ?? 0) / 100) * GAUGE_WIDTH),
                width: `${(range()[1] ?? 0) - (range()[0] ?? 0)}%`,
              }}
            />
          </Slider.Track>
          <Slider.Thumb scope="sl-range" index={0} style={thumb(range()[0] ?? 0)} />
          <Slider.Thumb scope="sl-range" index={1} style={thumb(range()[1] ?? 0)} />
        </Slider.Control>
      </Slider.Root>
      <Note text={`range [${range().join(", ")}]`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Splitter
 * -------------------------------------------------------------------------------------------- */

function SplitterDemo() {
  const [sizes, setSizes] = createSignal<readonly number[]>([200, 160, 160]);

  return (
    <Panel
      title="Splitter"
      hint="Pane sizes are driven entirely by onSizesChange: the core conserves the total, honours each pane's minimum, and collapses a collapsible pane."
    >
      <Splitter.Root
        scope="split"
        value={sizes()}
        step={8}
        panes={[{ min: 80 }, { min: 80 }, { min: 60, collapsible: true }]}
        onSizesChange={setSizes}
        style={{
          display: "flex",
          flexDirection: "row",
          height: 140,
          borderRadius: 10,
          borderWidth: 1,
          borderColor: p().border,
          overflow: "hidden",
          backgroundColor: p().panelAlt,
        }}
      >
        <For each={[0, 1, 2]}>
          {(index) => (
            <>
              <Show when={index > 0}>
                <Splitter.Handle
                  scope="split"
                  itemIndex={index - 1}
                  style={{
                    width: 6,
                    backgroundColor: p().border,
                    cursor: "col-resize",
                    focusOutline: `2px solid ${p().accent}`,
                  }}
                />
              </Show>
              <Splitter.Pane
                scope="split"
                itemIndex={index}
                style={{ alignItems: "center", justifyContent: "center" }}
              >
                <Muted text={`pane ${index} · ${Math.round(sizes()[index] ?? 0)}px`} />
              </Splitter.Pane>
            </>
          )}
        </For>
      </Splitter.Root>
      <Note text={`sizes [${sizes().map((size) => Math.round(size)).join(", ")}]`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Switch
 * -------------------------------------------------------------------------------------------- */

function SwitchDemo() {
  const [wifi, setWifi] = createSignal(true);
  const [beta, setBeta] = createSignal(false);
  const [managed, setManaged] = createSignal(true);

  const shell = (on: boolean) =>
    ({
      width: 44,
      height: 24,
      borderRadius: 12,
      padding: 2,
      display: "flex",
      flexDirection: "row",
      alignItems: "center",
      backgroundColor: on ? p().accent : p().track,
      focusOutline: `2px solid ${p().accent}`,
      outlineOffset: 2,
    }) as const;

  const knob = (on: boolean) =>
    ({
      width: 20,
      height: 20,
      borderRadius: 10,
      backgroundColor: "#ffffff",
      marginLeft: on ? 20 : 0,
    }) as const;

  return (
    <Panel
      title="Switch"
      hint="A switch is not a checkbox: the core exposes it with the Switch role and its own on/off state."
    >
      <Row>
        <Switch.Root checked={wifi()} onCheckedChange={setWifi} style={shell(wifi())}>
          <Switch.Thumb style={knob(wifi())} />
        </Switch.Root>
        <Label text="Wi-Fi" />
      </Row>
      <Row>
        <Switch.Root checked={beta()} onCheckedChange={setBeta} style={shell(beta())}>
          <Switch.Thumb style={knob(beta())} />
        </Switch.Root>
        <Label text="Beta updates" />
      </Row>
      <Row>
        <Switch.Root checked={managed()} readOnly style={shell(managed())}>
          <Switch.Thumb style={knob(managed())} />
        </Switch.Root>
        <Muted text="Managed by policy · read-only, still focusable" />
      </Row>
      <Note text={`wifi ${wifi()} · beta ${beta()} · managed ${managed()}`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Tabs
 * -------------------------------------------------------------------------------------------- */

function TabsDirection() {
  const tabs = useTabsState();
  const indicator = () => tabs().indicator;
  return (
    <Note
      text={`activation direction ${tabs().activationDirection}${indicator() ? ` · indicator ${Math.round(indicator()!.width)}×${Math.round(indicator()!.height)} at ${Math.round(indicator()!.left)}` : ""}`}
    />
  );
}

function TabsDemo() {
  const [tab, setTab] = createSignal("overview");

  const tabStyle = (value: string) =>
    ({
      ...controlStyle(),
      backgroundColor: tab() === value ? p().selection : "transparent",
      borderColor: "transparent",
    }) as const;

  return (
    <Panel
      title="Tabs"
      hint="This gallery's own sidebar is a vertical tab set. Here is the horizontal form, with automatic activation and an anchored indicator whose laid-out box the core publishes back."
    >
      <Tabs.Root
        value={tab()}
        onValueChange={setTab}
        activation="automatic"
        orientation="horizontal"
        style={{ display: "flex", flexDirection: "column", gap: 8 }}
      >
        <Tabs.List
          style={{
            display: "flex",
            flexDirection: "row",
            gap: 4,
            paddingBottom: 2,
            borderBottomWidth: 1,
            borderColor: p().border,
          }}
        >
          <Tabs.Tab value="overview" index={0} style={tabStyle("overview")}>
            <Text style={{ fontSize: 12, color: p().ink }}>Overview</Text>
          </Tabs.Tab>
          <Tabs.Tab value="usage" index={1} style={tabStyle("usage")}>
            <Text style={{ fontSize: 12, color: p().ink }}>Usage</Text>
          </Tabs.Tab>
          <Tabs.Tab value="limits" index={2} style={tabStyle("limits")}>
            <Text style={{ fontSize: 12, color: p().ink }}>Limits</Text>
          </Tabs.Tab>
          <Tabs.Indicator
            placement="bottom"
            style={{ height: 2, backgroundColor: p().accent }}
          />
        </Tabs.List>
        <TabsDirection />
        <Tabs.Panel value="overview">
          <Muted text="An inactive panel is not mounted at all, so it contributes no layout or paint." />
        </Tabs.Panel>
        <Tabs.Panel value="usage">
          <Muted text="Arrow keys move the tab stop; activation=automatic selects as focus moves." />
        </Tabs.Panel>
        <Tabs.Panel value="limits">
          <Muted text="Declare keepMounted to retain inactive panels as display:none instead." />
        </Tabs.Panel>
      </Tabs.Root>
      <Note text={`active ${tab()}`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Time field
 * -------------------------------------------------------------------------------------------- */

function TimeFieldDemo() {
  const [at, setAt] = createSignal("09:30");
  const [precise, setPrecise] = createSignal("14:05:30");

  const segment = () => ({ ...inputStyle(), width: 46, textAlign: "center" }) as const;

  return (
    <Panel
      title="Time field"
      hint="Civil times with no time zone. Without showSeconds the core owns no seconds segment at all."
    >
      <Row>
        <Muted text="12-hour" />
        <TimeField.Root
          scope="tf-at"
          value={at()}
          hour12
          onValueChange={(next) => setAt(next ?? "")}
          style={{ display: "flex", flexDirection: "row", gap: 4 }}
        >
          <TimeField.Segment scope="tf-at" segment="hour" style={segment()} />
          <TimeField.Segment scope="tf-at" segment="minute" style={segment()} />
          <TimeField.Segment scope="tf-at" segment="period" style={segment()} />
        </TimeField.Root>
      </Row>
      <Row>
        <Muted text="24-hour + seconds" />
        <TimeField.Root
          scope="tf-precise"
          value={precise()}
          showSeconds
          onValueChange={(next) => setPrecise(next ?? "")}
          style={{ display: "flex", flexDirection: "row", gap: 4 }}
        >
          <TimeField.Segment scope="tf-precise" segment="hour" style={segment()} />
          <TimeField.Segment scope="tf-precise" segment="minute" style={segment()} />
          <TimeField.Segment scope="tf-precise" segment="second" style={segment()} />
        </TimeField.Root>
      </Row>
      <Note text={`at ${at()} · precise ${precise()}`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Toast
 * -------------------------------------------------------------------------------------------- */

function ToastDemo() {
  return (
    <Toast.Provider timeout={5000} limit={3} pitch={10} swipeDirection="right">
      <ToastDemoBody />
    </Toast.Provider>
  );
}

function ToastDemoBody() {
  const toasts = useToastManager();
  let counter = 0;
  let lastId = "";

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
            lastId = toasts.add({
              title: `Build ${counter} queued`,
              description: "Auto-dismisses in 5s",
              type: "info",
            });
          }}
        />
        <Btn
          label="Success"
          onClick={() => {
            counter += 1;
            toasts.add({ title: `Build ${counter} finished`, type: "success" });
          }}
        />
        <Btn
          label="Error"
          onClick={() => {
            counter += 1;
            toasts.add({
              title: `Build ${counter} failed`,
              type: "error",
              duration: 8000,
            });
          }}
        />
        <Btn
          label="Update last"
          onClick={() => {
            if (lastId) toasts.update(lastId, { title: "Updated in place" });
          }}
        />
        <Btn label="Clear" onClick={() => toasts.closeAll()} />
      </Row>

      <Toast.Viewport
        style={{ display: "flex", flexDirection: "column", gap: 8, minHeight: 40 }}
      >
        <For each={toasts.stack()}>
          {(entry) => (
            <Toast.Positioner toastId={entry.id}>
              <Toast.Root
                toastId={entry.id}
                style={{
                  ...popupStyle(),
                  padding: 10,
                  gap: 4,
                  opacity: entry.limited ? 0.55 : 1,
                  transform: `translateX(${entry.swipeMovement}px)`,
                  borderColor:
                    entry.type === "error"
                      ? p().danger
                      : entry.type === "success"
                        ? p().accent
                        : p().border,
                }}
              >
                <Toast.Content toastId={entry.id}>
                  <Row>
                    <Toast.Title toastId={entry.id}>
                      <Text style={{ fontSize: 12, fontWeight: 700, color: p().ink }}>
                        {toasts.toasts().find((toast) => toast.id === entry.id)?.title ??
                          entry.type}
                      </Text>
                    </Toast.Title>
                    <Muted text={`#${entry.index} · +${entry.offset}px`} />
                    <Toast.Close toastId={entry.id}>
                      <Text style={{ fontSize: 13, color: p().muted }}>×</Text>
                    </Toast.Close>
                  </Row>
                  <Toast.Description toastId={entry.id}>
                    <Muted
                      text={
                        toasts.toasts().find((toast) => toast.id === entry.id)
                          ?.description ?? ""
                      }
                    />
                  </Toast.Description>
                </Toast.Content>
              </Toast.Root>
            </Toast.Positioner>
          )}
        </For>
      </Toast.Viewport>
      <Note
        text={`queued ${toasts.toasts().length} · stack ${toasts.stack().length} · limited ${toasts.stack().filter((entry) => entry.limited).length}`}
      />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Toggle
 * -------------------------------------------------------------------------------------------- */

function ToggleDemo() {
  const [bold, setBold] = createSignal(false);
  const [pinned, setPinned] = createSignal(true);

  const style = (on: boolean) =>
    ({
      ...controlStyle(),
      width: 44,
      paddingLeft: 0,
      paddingRight: 0,
      backgroundColor: on ? p().selection : p().control,
      borderColor: on ? p().accent : p().border,
    }) as const;

  return (
    <Panel
      title="Toggle"
      hint="A button that stays pressed, exposed as a toggle button rather than a checkbox."
    >
      <Row>
        <Toggle.Root pressed={bold()} onPressedChange={setBold} style={style(bold())}>
          <Text style={{ fontSize: 13, fontWeight: 700, color: p().ink }}>B</Text>
        </Toggle.Root>
        <Toggle.Root
          pressed={pinned()}
          onPressedChange={setPinned}
          style={{ ...style(pinned()), width: 90 }}
        >
          <Toggle.Indicator
            style={{
              width: 6,
              height: 6,
              borderRadius: 3,
              backgroundColor: pinned() ? p().accent : p().border,
            }}
          />
          <Text style={{ fontSize: 12, color: p().ink }}>Pinned</Text>
        </Toggle.Root>
      </Row>
      <Note text={`bold ${bold()} · pinned ${pinned()}`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Toggle group
 * -------------------------------------------------------------------------------------------- */

function ToggleGroupDemo() {
  const [align, setAlign] = createSignal<readonly string[]>(["left"]);
  const [formats, setFormats] = createSignal<readonly string[]>(["bold"]);

  const item = (on: boolean) =>
    ({
      ...controlStyle(),
      width: 46,
      paddingLeft: 0,
      paddingRight: 0,
      backgroundColor: on ? p().selection : p().control,
      borderColor: on ? p().accent : p().border,
    }) as const;

  return (
    <Panel
      title="Toggle group"
      hint="One roving Tab stop, wrapping arrow navigation, and disabled-item skipping, all decided by the core. single presses at most one item; multiple presses any number."
    >
      <Col>
        <Muted text="single" />
        <ToggleGroup.Root
          scope="tg-align"
          variant="single"
          items={[{ value: "left" }, { value: "center" }, { value: "right" }]}
          value={align()}
          onValueChange={setAlign}
          style={{ display: "flex", flexDirection: "row", gap: 4 }}
        >
          <For each={["left", "center", "right"]}>
            {(value) => (
              <ToggleGroup.Item
                scope="tg-align"
                partValue={value}
                style={item(align().includes(value))}
              >
                <Text style={{ fontSize: 11, color: p().ink }}>{value}</Text>
              </ToggleGroup.Item>
            )}
          </For>
        </ToggleGroup.Root>

        <Muted text="multiple, with one disabled item" />
        <ToggleGroup.Root
          scope="tg-format"
          variant="multiple"
          items={[
            { value: "bold" },
            { value: "italic" },
            { value: "underline", disabled: true },
          ]}
          value={formats()}
          onValueChange={setFormats}
          style={{ display: "flex", flexDirection: "row", gap: 4 }}
        >
          <ToggleGroup.Item
            scope="tg-format"
            partValue="bold"
            style={item(formats().includes("bold"))}
          >
            <Text style={{ fontSize: 13, fontWeight: 700, color: p().ink }}>B</Text>
          </ToggleGroup.Item>
          <ToggleGroup.Item
            scope="tg-format"
            partValue="italic"
            style={item(formats().includes("italic"))}
          >
            <Text style={{ fontSize: 13, color: p().ink }}>I</Text>
          </ToggleGroup.Item>
          <ToggleGroup.Item
            scope="tg-format"
            partValue="underline"
            style={{ ...item(false), opacity: 0.5 }}
          >
            <Text style={{ fontSize: 13, textDecoration: "underline", color: p().muted }}>
              U
            </Text>
          </ToggleGroup.Item>
        </ToggleGroup.Root>
      </Col>
      <Note text={`align [${align().join(", ")}] · formats [${formats().join(", ")}]`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Toolbar
 * -------------------------------------------------------------------------------------------- */

function ToolbarDemo() {
  const [tool, setTool] = createSignal("select");
  const [query, setQuery] = createSignal("");

  return (
    <Panel
      title="Toolbar"
      hint="One roving Tab stop across mixed item roles: items, a button, a link, and an input. Arrow navigation skips the disabled item; the core decides which one holds the Tab stop."
    >
      <Toolbar.Root
        scope="tb"
        orientation="horizontal"
        items={[
          { value: "select" },
          { value: "pen" },
          { value: "erase", disabled: true },
          { value: "share" },
          { value: "docs" },
          { value: "find" },
        ]}
        active={tool()}
        onActiveChange={(next) => setTool(next ?? "select")}
        style={{
          display: "flex",
          flexDirection: "row",
          alignItems: "center",
          gap: 4,
          padding: 5,
          borderRadius: 10,
          borderWidth: 1,
          borderColor: p().border,
          backgroundColor: p().panelAlt,
        }}
      >
        <Toolbar.Group scope="tb" style={{ display: "flex", flexDirection: "row", gap: 4 }}>
          <For each={["select", "pen", "erase"]}>
            {(value) => (
              <Toolbar.Item
                scope="tb"
                partValue={value}
                style={{
                  ...controlStyle(),
                  height: 26,
                  opacity: value === "erase" ? 0.5 : 1,
                  backgroundColor: tool() === value ? p().selection : p().control,
                }}
              >
                <Text style={{ fontSize: 11, color: p().ink }}>{value}</Text>
              </Toolbar.Item>
            )}
          </For>
        </Toolbar.Group>
        <Toolbar.Separator
          scope="tb"
          style={{ width: 1, height: 20, backgroundColor: p().border }}
        />
        <Toolbar.Button
          scope="tb"
          partValue="share"
          style={{ ...controlStyle(), height: 26 }}
        >
          <Text style={{ fontSize: 11, color: p().ink }}>Share</Text>
        </Toolbar.Button>
        <Toolbar.Link
          scope="tb"
          partValue="docs"
          style={{ ...controlStyle(), height: 26, borderColor: "transparent" }}
        >
          <Text style={{ fontSize: 11, color: p().accent }}>Docs</Text>
        </Toolbar.Link>
        <Toolbar.Input
          scope="tb"
          partValue="find"
          style={{ ...inputStyle(), height: 26, width: 140 }}
          onInput={(event) => setQuery(event.value ?? "")}
        />
      </Toolbar.Root>
      <Note text={`active ${tool()} · find "${query()}"`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Tooltip
 * -------------------------------------------------------------------------------------------- */

function TooltipDemo() {
  const [open, setOpen] = createSignal(false);
  const [placement, setPlacement] = createSignal("—");

  return (
    <Panel
      title="Tooltip"
      hint="A provider shares one warm group deadline across its triggers, so the second tooltip in a row opens without waiting again. Both are hoverable={false}, like a native help tag: the popup never takes the pointer, so reaching it closes the tooltip."
    >
      <Tooltip.Provider delay={500} closeDelay={120} timeout={400}>
        <Row>
          <Tooltip.Root
            side="top"
            sideOffset={8}
            hoverable={false}
            onOpenChange={setOpen}
            onPlacementChange={(details) =>
              setPlacement(`${details.side}/${details.align}`)
            }
          >
            <Tooltip.Trigger style={controlStyle()}>
              <Text style={{ fontSize: 12, color: p().ink }}>Hover me</Text>
            </Tooltip.Trigger>
            <Tooltip.Positioner>
              <Tooltip.Popup
                style={{
                  paddingLeft: 9,
                  paddingRight: 9,
                  paddingTop: 5,
                  paddingBottom: 5,
                  borderRadius: 7,
                  backgroundColor: p().ink,
                }}
              >
                <Text style={{ fontSize: 11, color: p().panel }}>
                  The core owns every deadline
                </Text>
                <Tooltip.Arrow
                  style={{ width: 8, height: 8, backgroundColor: p().ink }}
                />
              </Tooltip.Popup>
            </Tooltip.Positioner>
          </Tooltip.Root>

          <Tooltip.Root
            side="bottom"
            sideOffset={8}
            trackCursorAxis="x"
            hoverable={false}
          >
            <Tooltip.Trigger style={controlStyle()}>
              <Text style={{ fontSize: 12, color: p().ink }}>Tracks the cursor</Text>
            </Tooltip.Trigger>
            <Tooltip.Positioner>
              <Tooltip.Popup
                style={{
                  paddingLeft: 9,
                  paddingRight: 9,
                  paddingTop: 5,
                  paddingBottom: 5,
                  borderRadius: 7,
                  backgroundColor: p().ink,
                }}
              >
                <Text style={{ fontSize: 11, color: p().panel }}>trackCursorAxis="x"</Text>
              </Tooltip.Popup>
            </Tooltip.Positioner>
          </Tooltip.Root>
        </Row>
      </Tooltip.Provider>
      <Note text={`open ${open()} · resolved ${placement()}`} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Table
 * -------------------------------------------------------------------------------------------- */

function TableDemo() {
  const base = Array.from({ length: 5000 }, (_, index) => ({
    name: `asset-${String(index).padStart(4, "0")}.png`,
    size: (index % 900) + 12,
  }));
  const [sort, setSort] = createSignal<TableSortState | undefined>();
  const [range, setRange] = createSignal<VisibleRange>({ start: 0, end: 0 });
  const [selection, setSelection] = createSignal<readonly (readonly number[])[]>([]);
  const [activated, setActivated] = createSignal("nothing yet");
  const [widths, setWidths] = createSignal("declared");

  const ordered = () => {
    const order = sort();
    if (!order) return base;
    const sorted = [...base].sort((left, right) =>
      order.column === "size"
        ? left.size - right.size
        : left.name.localeCompare(right.name),
    );
    return order.direction === "descending" ? sorted.reverse() : sorted;
  };
  const visible = () => ordered().slice(range().start, range().end);
  const selected = () =>
    selection().reduce(
      (total, span) => total + ((span[1] ?? 0) - (span[0] ?? 0) + 1),
      0,
    );

  return (
    <Panel
      title="Table"
      hint="Five thousand rows, of which only the range the core last reported as visible is declared. Selection, sort, and column widths are all reported asynchronously."
    >
      <Table.Root
        scope="files"
        rowCount={base.length}
        rowHeight={26}
        headerHeight={28}
        selectionMode="multiple"
        selection={selection()}
        {...(sort() ? { sort: sort()! } : {})}
        columns={[
          { id: "name", label: "Name", width: 240, minWidth: 120, sortable: true, rowHeader: true },
          { id: "size", label: "Size", track: "1fr", align: "end", sortable: true },
        ]}
        onVisibleRangeChange={setRange}
        onSelectionChange={setSelection}
        onSortChange={setSort}
        onColumnResize={(next) =>
          setWidths(
            Object.entries(next)
              .map(([id, width]) => `${id} ${Math.round(width)}`)
              .join(", "),
          )
        }
        onActivate={(cell) => setActivated(`row ${cell.row}, column ${cell.column}`)}
        style={{
          height: 260,
          borderRadius: 10,
          borderWidth: 1,
          borderColor: p().border,
          backgroundColor: p().panelAlt,
          overflowY: "scroll",
        }}
      >
        <Table.Header column="name" style={{ paddingLeft: 10, justifyContent: "center" }}>
          <Text style={{ fontSize: 11, fontWeight: 700, color: p().faint }}>
            {`NAME${sort()?.column === "name" ? (sort()!.direction === "ascending" ? " ▲" : " ▼") : ""}`}
          </Text>
        </Table.Header>
        <Table.Header column="size" style={{ paddingRight: 10, justifyContent: "center" }}>
          <Text style={{ fontSize: 11, fontWeight: 700, color: p().faint }}>
            {`SIZE${sort()?.column === "size" ? (sort()!.direction === "ascending" ? " ▲" : " ▼") : ""}`}
          </Text>
        </Table.Header>
        <For each={visible()}>
          {(row, index) => (
            <Table.Row index={range().start + index()}>
              <Table.Cell column="name" style={{ paddingLeft: 10 }}>
                <Text style={{ fontSize: 12, color: p().ink }}>{row.name}</Text>
              </Table.Cell>
              <Table.Cell column="size" style={{ paddingRight: 10, justifyContent: "flex-end" }}>
                <Text style={{ fontSize: 12, color: p().muted, textAlign: "end" }}>
                  {`${row.size} KB`}
                </Text>
              </Table.Cell>
            </Table.Row>
          )}
        </For>
      </Table.Root>
      <Note
        text={`rows ${range().start}–${range().end} of ${base.length} · ${selected()} selected · sort ${sort() ? `${sort()!.column} ${sort()!.direction}` : "none"} · widths ${widths()} · activated ${activated()}`}
      />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Tree
 * -------------------------------------------------------------------------------------------- */

function TreeDemo() {
  const nodes: readonly TreeNodeDeclaration[] = [
    {
      id: "src",
      label: "src",
      children: [
        { id: "src/lib.rs", label: "lib.rs" },
        { id: "src/main.rs", label: "main.rs" },
        { id: "src/element", label: "element", pending: true },
      ],
    },
    {
      id: "docs",
      label: "docs",
      children: [
        { id: "docs/solid.md", label: "solid.md" },
        { id: "docs/tabs.md", label: "tabs.md" },
      ],
    },
  ];
  const [expanded, setExpanded] = createSignal<readonly string[]>(["src"]);
  const [selected, setSelected] = createSignal("—");
  const [activated, setActivated] = createSignal("nothing yet");
  const [range, setRange] = createSignal<VisibleRange>({ start: 0, end: 0 });
  const [loaded, setLoaded] = createSignal<
    { id: string; children: readonly TreeNodeDeclaration[] } | undefined
  >();
  const [asked, setAsked] = createSignal(0);

  // `setChildren` is declared only after the core asks for the branch, so the prop is absent
  // rather than explicitly undefined until then.
  const lazyChildren = () => {
    const splice = loaded();
    return splice ? { setChildren: splice } : {};
  };

  // The rows the core can mount, in tree order, each with the depth that indents it.
  type FlatRow = { node: TreeNodeDeclaration; depth: number };
  const flattened = (): readonly FlatRow[] => {
    const splice = loaded();
    const walk = (list: readonly TreeNodeDeclaration[], depth: number): FlatRow[] =>
      list.flatMap((node) => {
        const children =
          splice && splice.id === node.id ? splice.children : (node.children ?? []);
        return expanded().includes(node.id)
          ? [{ node, depth }, ...walk(children, depth + 1)]
          : [{ node, depth }];
      });
    return walk(nodes, 0);
  };
  const visible = () => flattened().slice(range().start, range().end);

  return (
    <Panel
      title="Tree"
      hint="A lazy tree. The one pending branch asks for its children exactly once through onLoadChildren, answered with a single validated setChildren splice."
    >
      <Tree.Root
        scope="explorer"
        nodes={nodes}
        rowHeight={26}
        loadingLabel="Loading…"
        disclosure="leading"
        expanded={expanded()}
        {...lazyChildren()}
        onExpandedChange={setExpanded}
        onValueChange={(next) => setSelected(next ?? "—")}
        onVisibleRangeChange={setRange}
        onActivate={setActivated}
        onLoadChildren={(id) => {
          setAsked(asked() + 1);
          setLoaded({
            id,
            children: [
              { id: `${id}/style.rs`, label: "style.rs" },
              { id: `${id}/state.rs`, label: "state.rs" },
            ],
          });
        }}
        style={{
          height: 220,
          borderRadius: 10,
          borderWidth: 1,
          borderColor: p().border,
          backgroundColor: p().panelAlt,
          overflowY: "scroll",
        }}
      >
        <For each={visible()}>
          {({ node, depth }) => (
            <Tree.Row
              nodeId={node.id}
              style={{
                display: "flex",
                flexDirection: "row",
                alignItems: "center",
                height: 26,
                paddingLeft: 10 + depth * 16,
              }}
            >
              <Text style={{ fontSize: 12, color: p().ink }}>
                {node.label ?? node.id}
              </Text>
            </Tree.Row>
          )}
        </For>
      </Tree.Root>
      <Note
        text={`expanded [${expanded().join(", ")}] · selected ${selected()} · activated ${activated()} · onLoadChildren asked ${asked()}×`}
      />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * The gallery shell
 * -------------------------------------------------------------------------------------------- */

interface Demo {
  id: string;
  label: string;
  /** Where the component's shape comes from: Base UI's catalog, or QuickGUI's own set. */
  source?: "QuickGUI";
  render: () => unknown;
}

/** The catalog a demo's parts and props follow. */
const sourceOf = (demo: Demo | undefined): string =>
  demo === undefined ? "" : demo.source === "QuickGUI" ? "QuickGUI component" : "Base UI part set";

const DEMOS: readonly Demo[] = [
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
  { id: "drawer", label: "Drawer", render: () => <DrawerDemo /> },
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

function Gallery() {
  const [tab, setTab] = createSignal(DEMOS[0]?.id ?? "accordion");
  const current = () => DEMOS.find((demo) => demo.id === tab());

  return (
    <View
      style={{
        display: "flex",
        flexDirection: "row",
        width: "100%",
        height: "100%",
        backgroundColor: p().window,
      }}
    >
      <Tabs.Root
        value={tab()}
        onValueChange={setTab}
        orientation="vertical"
        activation="manual"
        style={{
          display: "flex",
          flexDirection: "row",
          width: "100%",
          height: "100%",
        }}
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
          <View
            style={{
              display: "flex",
              flexDirection: "row",
              alignItems: "center",
              height: 52,
              flexShrink: 0,
              paddingLeft: 82,
              appRegion: "drag",
            }}
          >
            <Text style={{ fontSize: 13, fontWeight: 700, color: p().ink }}>
              Components
            </Text>
          </View>
          <View
            style={{
              display: "flex",
              flexDirection: "column",
              flex: 1,
              minHeight: 0,
              overflowY: "scroll",
            }}
          >
            <Tabs.List
              style={{
                display: "flex",
                flexDirection: "column",
                gap: 1,
                paddingLeft: 8,
                paddingRight: 8,
                paddingBottom: 12,
              }}
            >
              <For each={DEMOS}>
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
                      backgroundColor:
                        tab() === demo.id ? p().accent : "transparent",
                      hoverBackgroundColor:
                        tab() === demo.id ? p().accent : p().controlHover,
                      focusOutline: `2px solid ${p().accent}`,
                      outlineOffset: -2,
                    }}
                  >
                    <Text
                      style={{
                        fontSize: 12,
                        fontWeight: tab() === demo.id ? 600 : 400,
                        color: tab() === demo.id ? p().onAccent : p().ink,
                      }}
                    >
                      {demo.label}
                    </Text>
                  </Tabs.Tab>
                )}
              </For>
            </Tabs.List>
          </View>
        </View>

        {/* Content: the selected demo, in a scrollable pane. */}
        <View
          style={{
            display: "flex",
            flexDirection: "column",
            flex: 1,
            height: "100%",
          }}
        >
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
              <Text style={{ fontSize: 15, fontWeight: 700, color: p().ink }}>
                {current()?.label ?? ""}
              </Text>
              <Text style={{ fontSize: 11, color: p().faint }}>{sourceOf(current())}</Text>
            </View>
            <Text style={{ fontSize: 11, color: p().faint }}>
              {`${DEMOS.length} components · ${appearance()} appearance · ${Math.round(viewport().width)}×${Math.round(viewport().height)}`}
            </Text>
          </View>
          <View
            style={{
              flex: 1,
              overflowY: "scroll",
              display: "flex",
              flexDirection: "column",
              gap: 16,
              padding: 20,
            }}
          >
            <For each={DEMOS}>
              {(demo) => (
                <Show when={tab() === demo.id}>
                  <Tabs.Panel
                    value={demo.id}
                    style={{
                      display: "flex",
                      flexDirection: "column",
                      gap: 16,
                      maxWidth: 720,
                    }}
                  >
                    {demo.render() as never}
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
  return new Window({
    title: "QuickGUI Solid Components",
    width: 1080,
    height: 780,
    minimumWidth: 880,
    minimumHeight: 620,
    background: lightPalette.window,
    titleBarStyle: "hiddenInset",
    trafficLightPosition: { x: 16, y: 18 },
    renderer: createRenderer(() => <Gallery />),
  });
}

// The appearance is read from the window the core actually opened, and every later change is
// applied from the appearance callback — never from a component body or a memo.
Appearance.onChange((next) => setAppearance(next));

function trackWindow(window: Window): void {
  void window
    .getState()
    .then((state) => {
      setAppearance(state.appearance);
      setViewport(state.viewportSize);
    })
    .catch(() => {});
  window.onStateChange((state) => {
    setAppearance(state.appearance);
    setViewport(state.viewportSize);
  });
}

const mainWindow = openMainWindow();
trackWindow(mainWindow);

app.on("reopen", ({ hasVisibleWindows }) => {
  if (!hasVisibleWindows) trackWindow(openMainWindow());
});
