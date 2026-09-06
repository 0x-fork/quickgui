import type { NativeNode } from "@quickgui/native";
import { For, Text, View, createSignal, type Style } from "@quickgui/ui";
import { Radio, RadioGroup, Switch, Toggle, ToggleGroup } from "@quickgui/ui/controls";
import { Meter, Progress, useGaugeState } from "@quickgui/ui/gauges";
import { Slider, Splitter, useSliderState, type SplitterPaneDeclaration } from "@quickgui/ui/range";
import { Tabs, useTabsState } from "@quickgui/ui/tabs";
import { Separator, Toolbar } from "@quickgui/ui/toolbar";
import type { ComponentItem } from "@quickgui/ui/types";

import { GAUGE_WIDTH, controlStyle, inputStyle, p } from "../theme.ts";
import { Btn, Col, Label, Muted, Note, Panel, Row } from "../ui.tsx";

/* -------------------------------------------------------------------------------------------- *
 * Meter and progress
 * -------------------------------------------------------------------------------------------- */

function GaugeReadout(): NativeNode {
  const gauge = useGaugeState();
  return (
    <Note
      text={
        "status " + gauge().status + " · display " + (gauge().displayValue ?? "—") + " · completion " + (gauge().completion === null ? "—" : (gauge().completion ?? 0).toFixed(2))
      }
    />
  );
}

export function MeterDemo(): NativeNode {
  const [level, setLevel] = createSignal<number[]>([62]);
  const value = (): number => level()[0] ?? 0;
  const zone = (): string => (value() < 25 ? p().danger : value() > 80 ? "#c88a00" : p().accent);
  return (
    <Panel
      title="Meter"
      hint="A level inside a known range, not task progress. low/high/optimum let the application colour the gauge without the framework inventing thresholds."
    >
      <Meter.Root value={() => value()} min={0} max={100} low={25} high={80} optimum={50} format="percent" style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        <Row>
          <Meter.Label>
            <Muted text="Disk used" />
          </Meter.Label>
          <Meter.Value>
            <Text style={{ fontSize: 12, color: p().ink }}>{String(value()) + "%"}</Text>
          </Meter.Value>
        </Row>
        <Meter.Track style={{ display: "flex", flexDirection: "row", width: GAUGE_WIDTH, height: 8, borderRadius: 4, backgroundColor: p().track, overflow: "hidden" }}>
          <Meter.Indicator style={{ height: 8, borderRadius: 4, backgroundColor: zone(), width: String(value()) + "%" }} />
        </Meter.Track>
        <GaugeReadout />
      </Meter.Root>

      <Slider.Root
        value={level}
        min={0}
        max={100}
        step={1}
        onValueChange={(next) => setLevel(next)}
        style={{ display: "flex", flexDirection: "row", alignItems: "center", width: GAUGE_WIDTH, height: 22 }}
      >
        <Slider.Control style={{ position: "relative", display: "flex", flexDirection: "row", alignItems: "center", width: GAUGE_WIDTH, height: 22 }}>
          <Slider.Track style={{ display: "flex", flexDirection: "row", width: GAUGE_WIDTH, height: 5, borderRadius: 3, backgroundColor: p().track }}>
            <Slider.Indicator style={{ height: 5, borderRadius: 3, backgroundColor: p().accent, width: String(value()) + "%" }} />
          </Slider.Track>
          <Slider.Thumb
            index={0}
            style={{
              position: "absolute",
              left: Math.round((value() / 100) * GAUGE_WIDTH - 7),
              top: 4,
              width: 14,
              height: 14,
              borderRadius: 7,
              backgroundColor: p().accent,
              focus: { outline: "2px solid " + p().accent },
              outlineOffset: 2,
            }}
          />
        </Slider.Control>
      </Slider.Root>
    </Panel>
  );
}

export function ProgressDemo(): NativeNode {
  const total = 12;
  const [done, setDone] = createSignal(3);
  const [indeterminate, setIndeterminate] = createSignal(false);
  return (
    <Panel
      title="Progress"
      hint="The core derives the status (progressing, complete, or indeterminate) and formats the value. Indeterminate motion stays application-owned so no framework animation keeps the window awake."
    >
      <Progress.Root
        value={() => (indeterminate() ? undefined : done())}
        max={total}
        indeterminate={indeterminate}
        format="fraction"
        valueText={() => String(done()) + " of " + String(total) + " files"}
        style={{ display: "flex", flexDirection: "column", gap: 6 }}
      >
        <Row>
          <Progress.Label>
            <Muted text="Uploading" />
          </Progress.Label>
          <Progress.Value>
            <Text style={{ fontSize: 12, color: p().ink }}>{indeterminate() ? "…" : String(done()) + " / " + String(total)}</Text>
          </Progress.Value>
        </Row>
        <Progress.Track style={{ display: "flex", flexDirection: "row", width: GAUGE_WIDTH, height: 8, borderRadius: 4, backgroundColor: p().track, overflow: "hidden" }}>
          <Progress.Indicator
            style={{
              height: 8,
              borderRadius: 4,
              backgroundColor: p().accent,
              width: indeterminate() ? "35%" : String(Math.round((done() / total) * 100)) + "%",
            }}
          />
        </Progress.Track>
        <GaugeReadout />
      </Progress.Root>
      <Row>
        <Btn label="−1" onClick={() => setDone(Math.max(0, done() - 1))} />
        <Btn label="+1" onClick={() => setDone(Math.min(total, done() + 1))} />
        <Btn label="Complete" onClick={() => setDone(total)} />
        <Btn label={indeterminate() ? "Determinate" : "Indeterminate"} onClick={() => setIndeterminate(!indeterminate())} />
      </Row>
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Radio
 * -------------------------------------------------------------------------------------------- */

function radioRow(): Style {
  return {
    display: "flex",
    flexDirection: "row",
    alignItems: "center",
    gap: 8,
    height: 28,
    paddingLeft: 8,
    paddingRight: 12,
    borderRadius: 8,
    hover: { backgroundColor: p().controlHover },
    focus: { outline: "2px solid " + p().accent },
  };
}

function radioDot(on: boolean): Style {
  return {
    width: 15,
    height: 15,
    borderRadius: 8,
    borderWidth: on ? 4 : 1,
    borderColor: on ? p().accent : p().border,
    backgroundColor: p().control,
  };
}

const themeNames = ["light", "dark", "system"];
const lockedNames = ["a", "b", "c"];

export function RadioDemo(): NativeNode {
  const [theme, setTheme] = createSignal<string | undefined>("system");
  const [locked] = createSignal<string | undefined>("b");
  return (
    <Panel
      title="Radio"
      hint="Exactly one selection per group. Arrow keys move and select, the group keeps a single Tab stop, and readOnly refuses changes while staying focusable."
    >
      <RadioGroup.Root value={theme} onValueChange={(next) => setTheme(next)} required style={{ display: "flex", flexDirection: "column", gap: 2 }}>
        <For each={() => themeNames}>
          {(value) => (
            <Radio.Root value={value} style={radioRow()}>
              <Radio.Indicator style={radioDot(theme() === value)} />
              <Label text={value} />
            </Radio.Root>
          )}
        </For>
      </RadioGroup.Root>

      <Separator.Root style={{ height: 1, backgroundColor: p().border }} />

      <RadioGroup.Root value={locked} readOnly style={{ display: "flex", flexDirection: "row", gap: 8 }}>
        <For each={() => lockedNames}>
          {(value) => (
            <Radio.Root value={value} style={radioRow()}>
              <Radio.Indicator style={radioDot(locked() === value)} />
              <Muted text={"read-only " + value} />
            </Radio.Root>
          )}
        </For>
      </RadioGroup.Root>
      <Note text={"theme " + (theme() ?? "—") + " · read-only group " + (locked() ?? "—")} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Separator
 * -------------------------------------------------------------------------------------------- */

export function SeparatorDemo(): NativeNode {
  return (
    <Panel
      title="Separator"
      hint="Orientation and nothing else: no thickness, no colour, no inset. Never focusable, and never part of a neighbouring accessible name."
    >
      <Col gap={10}>
        <Label text="Above the rule" />
        <Separator.Root orientation="horizontal" style={{ height: 1, backgroundColor: p().border }} />
        <Label text="Below the rule" />
      </Col>
      <Row>
        <Muted text="Cut" />
        <Separator.Root orientation="vertical" style={{ width: 1, height: 18, backgroundColor: p().border }} />
        <Muted text="Copy" />
        <Separator.Root orientation="vertical" style={{ width: 1, height: 18, backgroundColor: p().border }} />
        <Muted text="Paste" />
      </Row>
      <Note text="an adjustable divider is Splitter, which is focusable and carries a value" />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Slider
 * -------------------------------------------------------------------------------------------- */

function SliderReadout(): NativeNode {
  const slider = useSliderState();
  return (
    <Text style={{ fontSize: 12, color: slider().dragging ? p().accent : p().muted, fontWeight: slider().dragging ? 700 : 400 }}>
      {slider().displayValue ?? "—"}
    </Text>
  );
}

// The look of an AppKit slider: a 4px track with the accent fill to the left of a 20px white
// knob that carries a hairline and a soft shadow. The whole 24px-tall control is the hit target:
// pressing anywhere in it jumps the nearest knob there and continues as a core-owned drag.
const KNOB = 20;

function sliderControl(): Style {
  return { position: "relative", display: "flex", flexDirection: "row", alignItems: "center", width: GAUGE_WIDTH, height: 24 };
}

/** The core owns the value; the application owns where the knob is painted. */
function thumbLeft(value: number): number {
  return Math.round((value / 100) * GAUGE_WIDTH - KNOB / 2);
}

function sliderTrack(): Style {
  return { display: "flex", flexDirection: "row", width: GAUGE_WIDTH, height: 4, borderRadius: 2, backgroundColor: p().track, borderWidth: 1, borderColor: p().border };
}

function sliderFill(): Style {
  return { height: 4, borderRadius: 2, marginTop: -1, marginLeft: -1, backgroundColor: p().accent };
}

function sliderThumb(value: number): Style {
  return {
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
    active: { backgroundColor: p().controlHover },
    focus: { outline: "2px solid " + p().accent },
    outlineOffset: 2,
  };
}

export function SliderDemo(): NativeNode {
  const [volume, setVolume] = createSignal<number[]>([40]);
  const [committed, setCommitted] = createSignal<number | undefined>(undefined);
  const [range, setRange] = createSignal<number[]>([20, 70]);
  const first = (values: number[]): number => values[0] ?? 0;
  const second = (values: number[]): number => values[1] ?? 0;
  return (
    <Panel
      title="Slider"
      hint="Clamping, step snapping, thumb ordering, and the captured pointer arithmetic are all the core's. onValueCommitted fires on the frame the gesture released."
    >
      <Slider.Root
        value={volume}
        min={0}
        max={100}
        step={5}
        largeStep={25}
        format="percent"
        onValueChange={(next) => setVolume(next)}
        onValueCommitted={(values) => setCommitted(values[0])}
        style={{ display: "flex", flexDirection: "column", gap: 6, width: GAUGE_WIDTH }}
      >
        <Row>
          <Slider.Label>
            <Muted text="Volume" />
          </Slider.Label>
          <Slider.Value>
            <SliderReadout />
          </Slider.Value>
        </Row>
        <Slider.Control style={sliderControl()}>
          <Slider.Track style={sliderTrack()}>
            <Slider.Indicator style={[sliderFill(), { width: String(first(volume())) + "%" }]} />
          </Slider.Track>
          <Slider.Thumb index={0} style={sliderThumb(first(volume()))} />
        </Slider.Control>
      </Slider.Root>
      <Note text={"volume " + String(first(volume())) + " · committed " + (committed() === undefined ? "—" : String(committed()))} />

      <Slider.Root
        value={range}
        min={0}
        max={100}
        step={1}
        minStepsBetweenValues={5}
        onValueChange={(next) => setRange(next)}
        style={{ display: "flex", flexDirection: "column", gap: 6, width: GAUGE_WIDTH }}
      >
        <Slider.Label>
          <Muted text="Range · the core holds five steps open between the thumbs" />
        </Slider.Label>
        <Slider.Control style={sliderControl()}>
          <Slider.Track style={sliderTrack()}>
            <Slider.Range
              style={[sliderFill(), { marginLeft: Math.round((first(range()) / 100) * GAUGE_WIDTH), width: String(second(range()) - first(range())) + "%" }]}
            />
          </Slider.Track>
          <Slider.Thumb index={0} style={sliderThumb(first(range()))} />
          <Slider.Thumb index={1} style={sliderThumb(second(range()))} />
        </Slider.Control>
      </Slider.Root>
      <Note text={"range [" + range().join(", ") + "]"} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Splitter
 * -------------------------------------------------------------------------------------------- */

const splitterPanes: SplitterPaneDeclaration[] = [{ min: 80 }, { min: 80 }, { min: 60, collapsible: true }];

export function SplitterDemo(): NativeNode {
  const [sizes, setSizes] = createSignal<number[]>([200, 160, 160]);
  const size = (index: number): number => Math.round(sizes()[index] ?? 0);
  const handle = (): Style => ({ width: 6, backgroundColor: p().border, cursor: "col-resize", focus: { outline: "2px solid " + p().accent } });
  const pane = (): Style => ({ alignItems: "center", justifyContent: "center" });
  return (
    <Panel
      title="Splitter"
      hint="Pane sizes are driven entirely by onSizesChange: the core conserves the total, honours each pane's minimum, and collapses a collapsible pane."
    >
      <Splitter.Root
        value={sizes}
        step={8}
        panes={splitterPanes}
        onSizesChange={(next) => setSizes(next)}
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
        <Splitter.Pane index={0} style={pane()}>
          <Muted text={"pane 0 · " + String(size(0)) + "px"} />
        </Splitter.Pane>
        <Splitter.Handle index={0} style={handle()} />
        <Splitter.Pane index={1} style={pane()}>
          <Muted text={"pane 1 · " + String(size(1)) + "px"} />
        </Splitter.Pane>
        <Splitter.Handle index={1} style={handle()} />
        <Splitter.Pane index={2} style={pane()}>
          <Muted text={"pane 2 · " + String(size(2)) + "px"} />
        </Splitter.Pane>
      </Splitter.Root>
      <Note text={"sizes [" + String(size(0)) + ", " + String(size(1)) + ", " + String(size(2)) + "]"} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Switch
 * -------------------------------------------------------------------------------------------- */

function switchShell(on: boolean): Style {
  return {
    width: 44,
    height: 24,
    borderRadius: 12,
    padding: 2,
    display: "flex",
    flexDirection: "row",
    alignItems: "center",
    backgroundColor: on ? p().accent : p().track,
    focus: { outline: "2px solid " + p().accent },
    outlineOffset: 2,
  };
}

function switchKnob(on: boolean): Style {
  return { width: 20, height: 20, borderRadius: 10, backgroundColor: "#ffffff", marginLeft: on ? 20 : 0 };
}

export function SwitchDemo(): NativeNode {
  const [wifi, setWifi] = createSignal(true);
  const [beta, setBeta] = createSignal(false);
  const [managed] = createSignal(true);
  return (
    <Panel title="Switch" hint="A switch is not a checkbox: the core exposes it with the Switch role and its own on/off state.">
      <Row>
        <Switch.Root checked={wifi} onCheckedChange={(next) => setWifi(next)} style={switchShell(wifi())}>
          <Switch.Thumb style={switchKnob(wifi())} />
        </Switch.Root>
        <Label text="Wi-Fi" />
      </Row>
      <Row>
        <Switch.Root checked={beta} onCheckedChange={(next) => setBeta(next)} style={switchShell(beta())}>
          <Switch.Thumb style={switchKnob(beta())} />
        </Switch.Root>
        <Label text="Beta updates" />
      </Row>
      <Row>
        <Switch.Root checked={managed} readOnly style={switchShell(managed())}>
          <Switch.Thumb style={switchKnob(managed())} />
        </Switch.Root>
        <Muted text="Managed by policy · read-only, still focusable" />
      </Row>
      <Note text={"wifi " + String(wifi()) + " · beta " + String(beta()) + " · managed " + String(managed())} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Tabs
 * -------------------------------------------------------------------------------------------- */

function TabsDirection(): NativeNode {
  const tabs = useTabsState();
  const indicatorText = (): string => {
    const indicator = tabs().indicator;
    if (indicator === null) return "";
    return " · indicator " + String(Math.round(indicator.width)) + "×" + String(Math.round(indicator.height)) + " at " + String(Math.round(indicator.left));
  };
  return <Note text={"activation direction " + tabs().activationDirection + indicatorText()} />;
}

export function TabsDemo(): NativeNode {
  const [tab, setTab] = createSignal<string | undefined>("overview");
  const tabStyle = (value: string): Style[] => [controlStyle(), { backgroundColor: tab() === value ? p().selection : "transparent", borderColor: "transparent" }];
  return (
    <Panel
      title="Tabs"
      hint="This gallery's own sidebar is a vertical tab set. Here is the horizontal form, with automatic activation and an anchored indicator whose laid-out box the core publishes back."
    >
      <Tabs.Root value={tab} onValueChange={(next) => setTab(next)} activation="automatic" orientation="horizontal" style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        <Tabs.List style={{ display: "flex", flexDirection: "row", gap: 4, paddingBottom: 2, borderBottomWidth: 1, borderColor: p().border }}>
          <Tabs.Tab value="overview" index={0} style={tabStyle("overview")}>
            <Text style={{ fontSize: 12, color: p().ink }}>Overview</Text>
          </Tabs.Tab>
          <Tabs.Tab value="usage" index={1} style={tabStyle("usage")}>
            <Text style={{ fontSize: 12, color: p().ink }}>Usage</Text>
          </Tabs.Tab>
          <Tabs.Tab value="limits" index={2} style={tabStyle("limits")}>
            <Text style={{ fontSize: 12, color: p().ink }}>Limits</Text>
          </Tabs.Tab>
          <Tabs.Indicator placement="bottom" style={{ height: 2, backgroundColor: p().accent }} />
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
      <Note text={"active " + (tab() ?? "—")} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Toggle
 * -------------------------------------------------------------------------------------------- */

function toggleStyle(on: boolean): Style[] {
  return [controlStyle(), { width: 44, paddingLeft: 0, paddingRight: 0, backgroundColor: on ? p().selection : p().control, borderColor: on ? p().accent : p().border }];
}

export function ToggleDemo(): NativeNode {
  const [bold, setBold] = createSignal(false);
  const [pinned, setPinned] = createSignal(true);
  return (
    <Panel title="Toggle" hint="A button that stays pressed, exposed as a toggle button rather than a checkbox.">
      <Row>
        <Toggle.Root pressed={bold} onPressedChange={(next) => setBold(next)} style={toggleStyle(bold())}>
          <Text style={{ fontSize: 13, fontWeight: 700, color: p().ink }}>B</Text>
        </Toggle.Root>
        <Toggle.Root pressed={pinned} onPressedChange={(next) => setPinned(next)} style={[...toggleStyle(pinned()), { width: 90 }]}>
          <Toggle.Indicator style={{ width: 6, height: 6, borderRadius: 3, backgroundColor: pinned() ? p().accent : p().border }} />
          <Text style={{ fontSize: 12, color: p().ink }}>Pinned</Text>
        </Toggle.Root>
      </Row>
      <Note text={"bold " + String(bold()) + " · pinned " + String(pinned())} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Toggle group
 * -------------------------------------------------------------------------------------------- */

const alignItems: ComponentItem[] = [{ value: "left" }, { value: "center" }, { value: "right" }];
const alignNames = ["left", "center", "right"];
const formatItems: ComponentItem[] = [{ value: "bold" }, { value: "italic" }, { value: "underline", disabled: true }];

function groupItem(on: boolean): Style[] {
  return [controlStyle(), { width: 46, paddingLeft: 0, paddingRight: 0, backgroundColor: on ? p().selection : p().control, borderColor: on ? p().accent : p().border }];
}

export function ToggleGroupDemo(): NativeNode {
  const [align, setAlign] = createSignal<string[]>(["left"]);
  const [formats, setFormats] = createSignal<string[]>(["bold"]);
  return (
    <Panel
      title="Toggle group"
      hint="One roving Tab stop, wrapping arrow navigation, and disabled-item skipping, all decided by the core. single presses at most one item; multiple presses any number."
    >
      <Col>
        <Muted text="single" />
        <ToggleGroup.Root items={alignItems} value={align} onValueChange={(next) => setAlign(next)} style={{ display: "flex", flexDirection: "row", gap: 4 }}>
          <For each={() => alignNames}>
            {(value) => (
              <ToggleGroup.Item value={value} style={groupItem(align().includes(value))}>
                <Text style={{ fontSize: 11, color: p().ink }}>{value}</Text>
              </ToggleGroup.Item>
            )}
          </For>
        </ToggleGroup.Root>

        <Muted text="multiple, with one disabled item" />
        <ToggleGroup.Root items={formatItems} multiple value={formats} onValueChange={(next) => setFormats(next)} style={{ display: "flex", flexDirection: "row", gap: 4 }}>
          <ToggleGroup.Item value="bold" style={groupItem(formats().includes("bold"))}>
            <Text style={{ fontSize: 13, fontWeight: 700, color: p().ink }}>B</Text>
          </ToggleGroup.Item>
          <ToggleGroup.Item value="italic" style={groupItem(formats().includes("italic"))}>
            <Text style={{ fontSize: 13, color: p().ink }}>I</Text>
          </ToggleGroup.Item>
          <ToggleGroup.Item value="underline" style={[...groupItem(false), { opacity: 0.5 }]}>
            <Text style={{ fontSize: 13, textDecoration: "underline", color: p().muted }}>U</Text>
          </ToggleGroup.Item>
        </ToggleGroup.Root>
      </Col>
      <Note text={"align [" + align().join(", ") + "] · formats [" + formats().join(", ") + "]"} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Toolbar
 * -------------------------------------------------------------------------------------------- */

const toolbarItems: ComponentItem[] = [
  { value: "select" },
  { value: "pen" },
  { value: "erase", disabled: true },
  { value: "share" },
  { value: "docs" },
  { value: "find" },
];
const toolNames = ["select", "pen", "erase"];

export function ToolbarDemo(): NativeNode {
  const [tool, setTool] = createSignal<string | undefined>("select");
  const [query, setQuery] = createSignal("");
  return (
    <Panel
      title="Toolbar"
      hint="One roving Tab stop across mixed item roles: items, a button, a link, and an input. Arrow navigation skips the disabled item; the core decides which one holds the Tab stop."
    >
      <Toolbar.Root
        orientation="horizontal"
        items={toolbarItems}
        active={tool}
        onActiveChange={(next) => setTool(next)}
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
        <Toolbar.Group style={{ display: "flex", flexDirection: "row", gap: 4 }}>
          <For each={() => toolNames}>
            {(value) => (
              <Toolbar.Item value={value} style={[controlStyle(), { height: 26, opacity: value === "erase" ? 0.5 : 1, backgroundColor: tool() === value ? p().selection : p().control }]}>
                <Text style={{ fontSize: 11, color: p().ink }}>{value}</Text>
              </Toolbar.Item>
            )}
          </For>
        </Toolbar.Group>
        <Toolbar.Separator style={{ width: 1, height: 20, backgroundColor: p().border }} />
        <Toolbar.Button value="share" style={[controlStyle(), { height: 26 }]}>
          <Text style={{ fontSize: 11, color: p().ink }}>Share</Text>
        </Toolbar.Button>
        <Toolbar.Link value="docs" style={[controlStyle(), { height: 26, borderColor: "transparent" }]}>
          <Text style={{ fontSize: 11, color: p().accent }}>Docs</Text>
        </Toolbar.Link>
        <Toolbar.Input partValue="find" style={[inputStyle(), { height: 26, width: 140 }]} onInput={(event) => setQuery(event.value ?? "")} />
      </Toolbar.Root>
      <Note text={"active " + (tool() ?? "—") + ' · find "' + query() + '"'} />
    </Panel>
  );
}
