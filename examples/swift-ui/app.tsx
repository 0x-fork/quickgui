import { app, Window, type NativeNode } from "@quickgui/native";
import {
  Button,
  Input,
  Text,
  View,
  createRenderer,
} from "@quickgui/ui";
import {
  Button as SwiftButton,
  ColorPicker as SwiftColorPicker,
  DatePicker as SwiftDatePicker,
  Gauge as SwiftGauge,
  Host,
  Picker as SwiftPicker,
  Popover,
  ProgressView as SwiftProgressView,
  QuickGUIHostView,
  SecureField as SwiftSecureField,
  SegmentedControl as SwiftSegmentedControl,
  Slider as SwiftSlider,
  Stepper as SwiftStepper,
  TextField as SwiftTextField,
  Toggle as SwiftToggle,
} from "@quickgui/ui/swift-ui";
import { buttonStyle, controlSize } from "@quickgui/ui/swift-ui/modifiers";
import { Tabs } from "@quickgui/ui/tabs";
import { createSignal, For, Show } from "@quickgui/ui";

await app.whenReady();

type DemoId =
  | "button"
  | "slider"
  | "toggle"
  | "progress-view"
  | "stepper"
  | "segmented-control"
  | "picker"
  | "date-picker"
  | "color-picker"
  | "gauge"
  | "text-field"
  | "secure-field"
  | "popover";

interface DemoDefinition {
  id: DemoId;
  label: string;
  description: string;
}

const DEMOS: DemoDefinition[] = [
  {
    id: "button",
    label: "Button",
    description:
      "A native SwiftUI button with an SF Symbol, Liquid Glass styling, and an asynchronous press event delivered to QuickGUI.",
  },
  {
    id: "slider",
    label: "Slider",
    description:
      "A controlled native slider with a bounded range and discrete steps. Its value is owned by the QuickGUI signal below.",
  },
  {
    id: "toggle",
    label: "Toggle",
    description:
      "A controlled SwiftUI toggle that reports its native on/off state through the hosted event queue.",
  },
  {
    id: "progress-view",
    label: "Progress View",
    description:
      "A determinate SwiftUI progress view with a semantic label and a formatted current-value label.",
  },
  {
    id: "stepper",
    label: "Stepper",
    description:
      "A bounded native stepper. SwiftUI performs the interaction and QuickGUI receives the updated numeric value.",
  },
  {
    id: "segmented-control",
    label: "Segmented Control",
    description:
      "A native segmented picker using the neutral tabs role—the Liquid Glass treatment used for Xcode-style navigation.",
  },
  {
    id: "picker",
    label: "Picker",
    description:
      "A native menu picker backed by typed options and a controlled string selection.",
  },
  {
    id: "date-picker",
    label: "Date Picker",
    description:
      "A native field-style date and time picker whose value crosses the bridge as a Unix timestamp in milliseconds.",
  },
  {
    id: "color-picker",
    label: "Color Picker",
    description:
      "A native color well with opacity support. SwiftUI selections are returned as RGBA hex strings.",
  },
  {
    id: "gauge",
    label: "Gauge",
    description:
      "A native accessory-capacity gauge with minimum, maximum, and current-value labels.",
  },
  {
    id: "text-field",
    label: "Text Field",
    description:
      "A controlled native text field that reports edits and Return-key submissions independently.",
  },
  {
    id: "secure-field",
    label: "Secure Field",
    description:
      "The secure variant uses SwiftUI's native concealed editor while keeping the same QuickGUI value contract.",
  },
  {
    id: "popover",
    label: "Popover",
    description:
      "A SwiftUI button presents a native popover, which reverse-hosts an ordinary interactive QuickGUI subtree.",
  },
];

function openMainWindow() {
  new Window({
    title: "QuickGUI SwiftUI Components",
    width: 920,
    height: 680,
    minimumWidth: 720,
    minimumHeight: 500,
    background: "transparent",
    vibrancy: "sidebar",
    visualEffectState: "followWindow",
    titleBarStyle: "hiddenInset",
    trafficLightPosition: { x: 16, y: 18 },
    renderer: createRenderer(() => <Gallery />),
  });
}

app.onReopen( ({ hasVisibleWindows }) => {
  if (!hasVisibleWindows) openMainWindow();
});
openMainWindow();

function DemoPage(props: {
  description: string;
  state: () => string;
  children: () => NativeNode;
}) {
  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        width: "100%",
        maxWidth: 680,
        gap: 18,
      }}
    >
      <Text style={{ color: "#5f6672", fontSize: 14, lineHeight: 21 }}>
        {props.description}
      </Text>

      <View
        style={{
          display: "flex",
          flexDirection: "column",
          width: "100%",
          minHeight: 250,
          padding: 22,
          gap: 16,
          borderWidth: 1,
          borderColor: "#dedfe3",
          borderRadius: 14,
          backgroundColor: "#ffffff",
        }}
      >
        <Text
          style={{
            color: "#858b96",
            fontSize: 11,
            fontWeight: 700,
            letterSpacing: 0.8,
          }}
        >
          LIVE SWIFTUI DEMO
        </Text>
        <View
          style={{
            display: "flex",
            flex: 1,
            minHeight: 170,
            width: "100%",
            alignItems: "center",
            justifyContent: "center",
          }}
        >
          {props.children()}
        </View>
      </View>

      <View
        style={{
          display: "flex",
          flexDirection: "row",
          alignItems: "center",
          justifyContent: "space-between",
          gap: 16,
          width: "100%",
          minHeight: 42,
          paddingLeft: 14,
          paddingRight: 14,
          borderRadius: 10,
          backgroundColor: "#eceef2",
        }}
      >
        <Text style={{ color: "#727985", fontSize: 11, fontWeight: 700 }}>
          NATIVE STATE
        </Text>
        <Text style={{ color: "#252a33", fontSize: 12 }}>{props.state()}</Text>
      </View>
    </View>
  );
}

function Gallery() {
  const [page, setPage] = createSignal<DemoId>("button");
  const [buttonPresses, setButtonPresses] = createSignal(0);
  const [open, setOpen] = createSignal(false);
  const [name, setName] = createSignal("Ada");
  const [password, setPassword] = createSignal("");
  const [submittedField, setSubmittedField] = createSignal("none");
  const [volume, setVolume] = createSignal(0.4);
  const [notifications, setNotifications] = createSignal(true);
  const [copies, setCopies] = createSignal(2);
  const [layout, setLayout] = createSignal("grid");
  const [interval, setPickerInterval] = createSignal("week");
  const [scheduledAt, setScheduledAt] = createSignal(Date.now());
  const [accent, setAccent] = createSignal("#3366ffff");
  const current = () => DEMOS.find((demo) => demo.id === page());

  function renderDemo(id: DemoId): NativeNode {
    switch (id) {
      case "button":
        return (
          <DemoPage
            description={current()?.description ?? ""}
            state={() => `${buttonPresses()} press${buttonPresses() === 1 ? "" : "es"}`}
          >
            <Host matchContents>
              <SwiftButton
                label="Continue"
                systemImage="arrow.right"
                modifiers={[buttonStyle("glass"), controlSize("large")]}
                onPress={() => setButtonPresses(buttonPresses() + 1)}
              />
            </Host>
          </DemoPage>
        );
      case "slider":
        return (
          <DemoPage
            description={current()?.description ?? ""}
            state={() => `volume ${Math.round(volume() * 100)}%`}
          >
            <Host matchContents={{ vertical: true }} style={{ width: 440 }}>
              <SwiftSlider
                label={`Volume ${Math.round(volume() * 100)}%`}
                value={volume()}
                min={0}
                max={1}
                step={0.05}
                onValueChange={setVolume}
              />
            </Host>
          </DemoPage>
        );
      case "toggle":
        return (
          <DemoPage
            description={current()?.description ?? ""}
            state={() => (notifications() ? "notifications on" : "notifications off")}
          >
            <Host matchContents>
              <SwiftToggle
                label="Notifications"
                isOn={notifications()}
                onIsOnChange={setNotifications}
              />
            </Host>
          </DemoPage>
        );
      case "progress-view":
        return (
          <DemoPage
            description={current()?.description ?? ""}
            state={() => `progress ${Math.round(volume() * 100)}%`}
          >
            <Host matchContents={{ vertical: true }} style={{ width: 440 }}>
              <SwiftProgressView
                label="Setup progress"
                value={volume()}
                total={1}
                currentValueLabel={`${Math.round(volume() * 100)}%`}
              />
            </Host>
          </DemoPage>
        );
      case "stepper":
        return (
          <DemoPage
            description={current()?.description ?? ""}
            state={() => `${copies()} copies`}
          >
            <Host matchContents>
              <SwiftStepper
                label={`Copies: ${copies()}`}
                value={copies()}
                min={1}
                max={10}
                onValueChange={setCopies}
              />
            </Host>
          </DemoPage>
        );
      case "segmented-control":
        return (
          <DemoPage
            description={current()?.description ?? ""}
            state={() => `layout ${layout()}`}
          >
            <Host matchContents={{ vertical: true }} style={{ width: 340 }}>
              <SwiftSegmentedControl
                role="tabs"
                selection={layout()}
                options={[
                  { value: "list", label: "List" },
                  { value: "grid", label: "Grid" },
                ]}
                onSelectionChange={setLayout}
              />
            </Host>
          </DemoPage>
        );
      case "picker":
        return (
          <DemoPage
            description={current()?.description ?? ""}
            state={() => `interval ${interval()}`}
          >
            <Host matchContents>
              <SwiftPicker
                label="Report interval"
                selection={interval()}
                options={[
                  { value: "day", label: "Daily" },
                  { value: "week", label: "Weekly" },
                  { value: "month", label: "Monthly" },
                ]}
                style="menu"
                onSelectionChange={setPickerInterval}
              />
            </Host>
          </DemoPage>
        );
      case "date-picker":
        return (
          <DemoPage
            description={current()?.description ?? ""}
            state={() => new Date(scheduledAt()).toISOString()}
          >
            <Host matchContents>
              <SwiftDatePicker
                label="Schedule"
                value={scheduledAt()}
                displayedComponents="dateAndTime"
                style="field"
                onValueChange={setScheduledAt}
              />
            </Host>
          </DemoPage>
        );
      case "color-picker":
        return (
          <DemoPage
            description={current()?.description ?? ""}
            state={() => `accent ${accent()}`}
          >
            <Host matchContents>
              <SwiftColorPicker
                label="Accent"
                selection={accent()}
                onSelectionChange={setAccent}
              />
            </Host>
          </DemoPage>
        );
      case "gauge":
        return (
          <DemoPage
            description={current()?.description ?? ""}
            state={() => `value ${Math.round(volume() * 100)}%`}
          >
            <Host matchContents={{ vertical: true }} style={{ width: 440 }}>
              <SwiftGauge
                label="Volume"
                value={volume()}
                min={0}
                max={1}
                currentValueLabel={`${Math.round(volume() * 100)}%`}
                minimumValueLabel="0%"
                maximumValueLabel="100%"
                style="accessoryLinearCapacity"
              />
            </Host>
          </DemoPage>
        );
      case "text-field":
        return (
          <DemoPage
            description={current()?.description ?? ""}
            state={() => `value “${name()}” · last submitted ${submittedField()}`}
          >
            <Host matchContents={{ vertical: true }} style={{ width: 360 }}>
              <SwiftTextField
                value={name()}
                placeholder="Name"
                onValueChange={setName}
                onSubmit={() => setSubmittedField("text field")}
              />
            </Host>
            {/* Focus is shared with the native field: only one of the two shows it at a time. */}
            <Input
              value={name()}
              placeholder="Framework input bound to the same value"
              onInput={(event) => setName(event.value ?? "")}
              style={{
                width: 360,
                height: 28,
                flexShrink: 0,
                paddingLeft: 8,
                paddingRight: 8,
                color: "#111827",
                backgroundColor: "#ffffff",
                borderWidth: 1,
                borderColor: "#d1d5db",
                borderRadius: 6,
                focus: { borderColor: "#2563eb", outline: "3px solid #2563eb55" },
              }}
            />
          </DemoPage>
        );
      case "secure-field":
        return (
          <DemoPage
            description={current()?.description ?? ""}
            state={() => `${password().length} characters · last submitted ${submittedField()}`}
          >
            <Host matchContents={{ vertical: true }} style={{ width: 360 }}>
              <SwiftSecureField
                value={password()}
                placeholder="Password"
                onValueChange={setPassword}
                onSubmit={() => setSubmittedField("secure field")}
              />
            </Host>
          </DemoPage>
        );
      case "popover":
        return (
          <DemoPage
            description={current()?.description ?? ""}
            state={() => (open() ? "popover presented" : "popover dismissed")}
          >
            <Host matchContents>
              <Popover.Root
                isPresented={open()}
                onIsPresentedChange={setOpen}
                attachmentAnchor="bottom"
                arrowEdge="top"
              >
                <Popover.Trigger
                  render={
                    <SwiftButton
                      label="Open QuickGUI popover"
                      modifiers={[buttonStyle("glass"), controlSize("large")]}
                    />
                  }
                />
                <Popover.Content>
                  <QuickGUIHostView width={300} height={200}>
                    <View
                      style={{
                        display: "flex",
                        flexDirection: "column",
                        width: 300,
                        height: "100%",
                        padding: 20,
                        gap: 12,
                        overflowY: "auto",
                        backgroundColor: "transparent",
                      }}
                    >
                      <Text style={{ color: "#111827", fontSize: 15, flexShrink: 0 }}>
                        QuickGUI inside SwiftUI
                      </Text>
                      <Input
                        value={name()}
                        onInput={(event) => setName(event.value ?? "")}
                        style={{
                          width: "100%",
                          height: 36,
                          flexShrink: 0,
                          padding: 8,
                          color: "#111827",
                          backgroundColor: "#ffffff",
                          borderWidth: 1,
                          borderColor: "#d1d5db",
                          borderRadius: 8,
                        }}
                      />
                      <Button
                        onClick={() => setOpen(false)}
                        style={{
                          width: "100%",
                          height: 34,
                          flexShrink: 0,
                          padding: 8,
                          color: "#ffffff",
                          backgroundColor: "#2563eb",
                          borderRadius: 8,
                          justifyContent: "center",
                        }}
                      >
                        Save {name()}
                      </Button>
                    </View>
                  </QuickGUIHostView>
                </Popover.Content>
              </Popover.Root>
            </Host>
          </DemoPage>
        );
    }
  }

  return (
    <View
      style={{
        display: "flex",
        flexDirection: "row",
        width: "100%",
        height: "100%",
        backgroundColor: "transparent",
      }}
    >
      <Tabs.Root
        value={page()}
        onValueChange={(value) => setPage(value as DemoId)}
        orientation="vertical"
        activation="manual"
        style={{
          display: "flex",
          flexDirection: "row",
          width: "100%",
          height: "100%",
        }}
      >
        <View
          style={{
            display: "flex",
            flexDirection: "column",
            width: 220,
            height: "100%",
            flexShrink: 0,
            backgroundColor: "transparent",
            borderRightWidth: 1,
            borderColor: "#c9cbd0",
          }}
        >
          <View
            style={{
              display: "flex",
              flexDirection: "row",
              alignItems: "center",
              height: 54,
              flexShrink: 0,
              paddingLeft: 82,
              appRegion: "drag",
            }}
          >
            <Text style={{ color: "#252a33", fontSize: 13, fontWeight: 700 }}>
              SwiftUI
            </Text>
          </View>

          <Text
            style={{
              flexShrink: 0,
              paddingLeft: 18,
              paddingBottom: 7,
              color: "#747b87",
              fontSize: 10,
              fontWeight: 700,
              letterSpacing: 0.7,
            }}
          >
            COMPONENTS
          </Text>

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
                gap: 2,
                paddingLeft: 9,
                paddingRight: 9,
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
                      height: 29,
                      flexShrink: 0,
                      paddingLeft: 10,
                      paddingRight: 10,
                      borderRadius: 7,
                      cursor: "default",
                      userSelect: "none",
                      backgroundColor: page() === demo.id ? "#2878d4" : "transparent",
                      hoverBackgroundColor: page() === demo.id ? "#2878d4" : "#ffffff66",
                      focusOutline: "2px solid #2878d4",
                      outlineOffset: -2,
                    }}
                  >
                    <Text
                      style={{
                        color: page() === demo.id ? "#ffffff" : "#303641",
                        fontSize: 12,
                        fontWeight: page() === demo.id ? 600 : 400,
                      }}
                    >
                      {demo.label}
                    </Text>
                  </Tabs.Tab>
                )}
              </For>
            </Tabs.List>
          </View>

          <View
            style={{
              flexShrink: 0,
              padding: 13,
              borderTopWidth: 1,
              borderColor: "#c9cbd0",
            }}
          >
            <Text style={{ color: "#747b87", fontSize: 11 }}>
              {`${DEMOS.length} native components`}
            </Text>
          </View>
        </View>

        <View
          style={{
            display: "flex",
            flexDirection: "column",
            flex: 1,
            minWidth: 0,
            height: "100%",
            backgroundColor: "#f6f6f8",
          }}
        >
          <View
            style={{
              display: "flex",
              flexDirection: "row",
              alignItems: "center",
              justifyContent: "space-between",
              height: 54,
              flexShrink: 0,
              paddingLeft: 22,
              paddingRight: 22,
              borderBottomWidth: 1,
              borderColor: "#d7d8dc",
              appRegion: "drag",
            }}
          >
            <Text style={{ color: "#20242c", fontSize: 15, fontWeight: 700 }}>
              {current()?.label ?? ""}
            </Text>
            <Text style={{ color: "#858b96", fontSize: 11 }}>Native SwiftUI · QuickGUI state</Text>
          </View>

          <View
            style={{
              display: "flex",
              flexDirection: "column",
              flex: 1,
              minHeight: 0,
              alignItems: "center",
              overflowY: "scroll",
              padding: 30,
            }}
          >
            <For each={DEMOS}>
              {(demo) => (
                <Show when={page() === demo.id}>
                  <Tabs.Panel
                    value={demo.id}
                    style={{
                      display: "flex",
                      flexDirection: "column",
                      alignItems: "center",
                      width: "100%",
                    }}
                  >
                    {renderDemo(demo.id)}
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
