import { app, Window } from "@quickgui/native";
import { Button, Popover, SystemPopover, Text, View, createRenderer } from "@quickgui/solid";
import { createSignal } from "solid-js";

await app.whenReady();

const buttonStyle = {
  display: "flex",
  width: "100%",
  height: 42,
  alignItems: "center",
  justifyContent: "center",
  paddingLeft: 16,
  paddingRight: 16,
  backgroundColor: "#2563eb",
  color: "#ffffff",
  borderRadius: 9,
  cursor: "default",
  appRegion: "no-drag",
  userSelect: "none",
} as const;

const secondaryButtonStyle = {
  ...buttonStyle,
  backgroundColor: "#30394a",
  borderColor: "#465166",
  borderWidth: 1,
} as const;

function PopoverContent(props: {
  kind: "System popover" | "In-window popover";
  description: string;
  onClose: () => void;
}) {
  const [count, setCount] = createSignal(0);

  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        width: "100%",
        height: "100%",
        gap: 14,
        padding: 20,
        backgroundColor: "#151a23",
        color: "#f5f7fb",
        borderColor: "#3b4558",
        borderWidth: 1,
        borderRadius: 12,
      }}
    >
      <View style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        <Text style={{ color: "#93c5fd", fontSize: 12, fontWeight: 700 }}>{props.kind}</Text>
        <Text style={{ fontSize: 19, lineHeight: 24, fontWeight: 700 }}>
          Interactive popover content
        </Text>
        <Text style={{ color: "#aeb8c9", fontSize: 13, lineHeight: 19 }}>
          {props.description}
        </Text>
      </View>

      <View style={{ display: "flex", flexDirection: "row", gap: 10 }}>
        <Button
          onClick={() => setCount((value) => value + 1)}
          style={{ ...secondaryButtonStyle, flex: 1, width: 0 }}
        >
          Count: {count()}
        </Button>
        <Button onClick={props.onClose} style={{ ...secondaryButtonStyle, flex: 1, width: 0 }}>
          Close
        </Button>
      </View>
    </View>
  );
}

function PopoverExample() {
  const [status, setStatus] = createSignal("Open either surface to compare its native behavior.");
  const [systemOpen, setSystemOpen] = createSignal(false);
  const [inWindowOpen, setInWindowOpen] = createSignal(false);

  function setSystemPopoverOpen(open: boolean) {
    setSystemOpen(open);
    setStatus(open ? "System popover opened." : "System popover closed.");
  }

  function closeSystemPopover() {
    setSystemOpen(false);
    setStatus("System popover closed.");
  }

  function setInWindowPopoverOpen(open: boolean) {
    setInWindowOpen(open);
    setStatus(open ? "In-window popover opened." : "In-window popover closed.");
  }

  function closeInWindowPopover() {
    setInWindowOpen(false);
    setStatus("In-window popover closed.");
  }

  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        width: "100%",
        height: "100%",
        backgroundColor: "#0b0f17",
        color: "#f5f7fb",
      }}
    >
      <View
        style={{
          display: "flex",
          height: 52,
          flexShrink: 0,
          alignItems: "center",
          justifyContent: "center",
          appRegion: "drag",
          borderColor: "#202838",
          borderWidth: 1,
        }}
      >
        <Text style={{ fontSize: 14, fontWeight: 600 }}>Solid native popover surfaces</Text>
      </View>

      <View
        style={{
          display: "flex",
          flex: 1,
          minHeight: 0,
          alignItems: "center",
          justifyContent: "center",
          padding: 36,
        }}
      >
        <View
          style={{
            display: "flex",
            flexDirection: "column",
            width: "100%",
            maxWidth: 680,
            gap: 20,
          }}
        >
          <View style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <Text style={{ fontSize: 28, lineHeight: 34, fontWeight: 700 }}>
              System and in-window popovers
            </Text>
            <Text style={{ color: "#9ba8bc", fontSize: 14, lineHeight: 21 }}>
              Both use the same controlled JSX API. SystemPopover portals into a native child
              window; Popover stays in this window's retained overlay plane.
            </Text>
          </View>

          <View style={{ display: "flex", flexDirection: "row", gap: 14 }}>
            <View
              style={{
                display: "flex",
                flexDirection: "column",
                flex: 1,
                minWidth: 0,
                gap: 14,
                padding: 20,
                backgroundColor: "#151a23",
                borderColor: "#30394a",
                borderWidth: 1,
                borderRadius: 12,
              }}
            >
              <Text style={{ fontSize: 17, fontWeight: 700 }}>SystemPopover</Text>
              <Text style={{ color: "#9ba8bc", fontSize: 13, lineHeight: 19 }}>
                A child window that may cross the owner's edge and stays within the display.
              </Text>
              <SystemPopover.Root
                open={systemOpen()}
                onOpenChange={(open) => setSystemPopoverOpen(open)}
              >
                <SystemPopover.Trigger style={buttonStyle}>
                  {systemOpen() ? "Close system" : "Open system"}
                </SystemPopover.Trigger>
                <SystemPopover.Content
                  width={340}
                  height={220}
                  placement="bottom-start"
                  gap={8}
                  viewportMargin={12}
                >
                  <PopoverContent
                    kind="System popover"
                    description="It renders through a separate Solid renderer on a native child surface and may cross the owner window's edge."
                    onClose={closeSystemPopover}
                  />
                </SystemPopover.Content>
              </SystemPopover.Root>
            </View>

            <View
              style={{
                display: "flex",
                flexDirection: "column",
                flex: 1,
                minWidth: 0,
                gap: 14,
                padding: 20,
                backgroundColor: "#151a23",
                borderColor: "#30394a",
                borderWidth: 1,
                borderRadius: 12,
              }}
            >
              <Text style={{ fontSize: 17, fontWeight: 700 }}>In-window popover</Text>
              <Text style={{ color: "#9ba8bc", fontSize: 13, lineHeight: 19 }}>
                A retained overlay that flips and shifts but remains inside this window.
              </Text>
              <Popover.Root
                open={inWindowOpen()}
                onOpenChange={(open) => setInWindowPopoverOpen(open)}
              >
                <Popover.Trigger style={buttonStyle}>
                  {inWindowOpen() ? "Close in-window" : "Open in-window"}
                </Popover.Trigger>
                <Popover.Content
                  width={340}
                  height={220}
                  placement="bottom-start"
                  gap={8}
                  viewportMargin={12}
                >
                  <PopoverContent
                    kind="In-window popover"
                    description="It shares this Solid root and renders above ordinary content without creating another native window."
                    onClose={closeInWindowPopover}
                  />
                </Popover.Content>
              </Popover.Root>
            </View>
          </View>

          <View
            style={{
              display: "flex",
              minHeight: 50,
              alignItems: "center",
              justifyContent: "center",
              paddingLeft: 16,
              paddingRight: 16,
              backgroundColor: "#10151e",
              borderColor: "#293244",
              borderWidth: 1,
              borderRadius: 9,
            }}
          >
            <Text style={{ color: "#b8c4d6", fontSize: 13 }}>{status()}</Text>
          </View>
        </View>
      </View>
    </View>
  );
}

function openMainWindow() {
  new Window({
    title: "QuickGUI Solid Popovers",
    width: 760,
    height: 540,
    minimumWidth: 620,
    minimumHeight: 480,
    background: "#0b0f17",
    titleBarStyle: "hiddenInset",
    trafficLightPosition: { x: 16, y: 14 },
    renderer: createRenderer(() => <PopoverExample />),
  });
}

app.on("reopen", ({ hasVisibleWindows }) => {
  if (!hasVisibleWindows) openMainWindow();
});
openMainWindow();
