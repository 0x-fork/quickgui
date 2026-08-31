import { app, Window } from "@quickgui/native";
import {
  Button,
  Input,
  Text,
  View,
  createRenderer,
} from "@quickgui/solid";
import {
  Button as SwiftButton,
  Host,
  Popover,
  QuickGUIHostView,
} from "@quickgui/solid/swift-ui";
import { buttonStyle, controlSize } from "@quickgui/solid/swift-ui/modifiers";
import { createSignal } from "solid-js";

await app.whenReady();

new Window({
  title: "QuickGUI SwiftUI",
  width: 480,
  height: 320,
  minimumWidth: 320,
  minimumHeight: 220,
  background: "#ffffff",
  renderer: createRenderer(() => <Demo />),
});

function Demo() {
  const [open, setOpen] = createSignal(false);
  const [name, setName] = createSignal("Ada");

  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        width: "100%",
        height: "100%",
        backgroundColor: "#ffffff",
      }}
    >
      <View
        style={{
          flexShrink: 0,
          justifyContent: "center",
          height: 40,
          alignItems: "center",
          backgroundColor: "#f0f0f0",
        }}
      >
        <Text>This is a QuickGUI component</Text>
      </View>
      <View
        style={{
          display: "flex",
          flexGrow: 1,
          width: "100%",
          minHeight: 0,
          alignItems: "center",
          justifyContent: "center",
          backgroundColor: "#ffffff",
        }}
      >
        <Host matchContents>
          <Popover
            isPresented={open()}
            onIsPresentedChange={setOpen}
            attachmentAnchor="bottom"
            arrowEdge="top"
          >
            <Popover.Trigger
              render={
                <SwiftButton
                  label="This is a SwiftUI button"
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
                  <Text
                    style={{ color: "#111827", fontSize: 15, flexShrink: 0 }}
                  >
                    This is QuickGUI component inside Swift UI Popover
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
          </Popover>
        </Host>
      </View>
    </View>
  );
}
