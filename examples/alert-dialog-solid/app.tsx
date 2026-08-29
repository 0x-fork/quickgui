import { App, Button, Dialog, Text, View, Window, render } from "@quickgui/solid";
import { createSignal } from "solid-js";

const app = new App();
const mainWindow = new Window({
  title: "QuickGUI Alert Dialogs",
  width: 680,
  height: 500,
  minimumWidth: 540,
  minimumHeight: 420,
  background: "#0b0e14",
  titleBarStyle: "hiddenInset",
  trafficLightPosition: { x: 16, y: 14 },
});

const buttonStyle = {
  display: "flex",
  height: 42,
  alignItems: "center",
  justifyContent: "center",
  paddingLeft: 16,
  paddingRight: 16,
  backgroundColor: "#262c38",
  color: "#f4f7fb",
  borderColor: "#3a4353",
  borderWidth: 1,
  borderRadius: 9,
  cursor: "default",
  appRegion: "no-drag",
  userSelect: "none",
} as const;

function AlertDialogExample() {
  const [pending, setPending] = createSignal(false);
  const [status, setStatus] = createSignal("Choose a dialog to present.");

  async function showInformationDialog() {
    await present(async () => {
      await Dialog.showAlertDialog({
        message: "QuickGUI uses a native alert dialog.",
        detail: "This call omits a parent window, so the alert is application-modal.",
      });
      return "Information dialog dismissed.";
    });
  }

  async function showSaveDialog() {
    await present(async () => {
      const response = await Dialog.showAlertDialog(mainWindow, {
        level: "warning",
        message: "Save changes before closing?",
        detail: "Your edits will be lost if you close this document without saving.",
        buttons: [
          { label: "Save", role: "default" },
          "Don't Save",
          { label: "Cancel", role: "cancel" },
        ],
      });
      return `Save dialog result: ${["Save", "Don't Save", "Cancel"][response] ?? response}`;
    });
  }

  async function showCriticalDialog() {
    await present(async () => {
      const response = await Dialog.showAlertDialog(mainWindow, {
        level: "critical",
        message: "Delete this workspace?",
        detail: "This example does not delete anything; it only demonstrates critical styling.",
        buttons: [
          { label: "Delete", role: "default" },
          { label: "Cancel", role: "cancel" },
        ],
      });
      return response === 0 ? "Delete selected (no action taken)." : "Delete cancelled.";
    });
  }

  async function present(action: () => Promise<string>) {
    if (pending()) return;
    setPending(true);
    setStatus("Waiting for the alert dialog…");
    try {
      setStatus(await action());
    } catch (error) {
      setStatus(`Dialog failed: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setPending(false);
    }
  }

  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        width: "100%",
        height: "100%",
        backgroundColor: "#0b0e14",
        color: "#f4f7fb",
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
          borderColor: "#1f2530",
          borderWidth: 1,
        }}
      >
        <Text style={{ fontSize: 14, fontWeight: 600 }}>Native alert dialogs</Text>
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
            width: 480,
            gap: 20,
            padding: 28,
            backgroundColor: "#151922",
            borderColor: "#2c3442",
            borderWidth: 1,
            borderRadius: 16,
          }}
        >
          <View style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <Text style={{ fontSize: 26, lineHeight: 32, fontWeight: 700 }}>System-owned UI</Text>
            <Text style={{ color: "#9aa6b7", fontSize: 14, lineHeight: 21 }}>
              Call Dialog.showAlertDialog from a Solid event handler, then await the selected button.
            </Text>
          </View>

          <View style={{ display: "flex", gap: 10 }}>
            <Button disabled={pending()} onClick={() => void showInformationDialog()} style={buttonStyle}>
              Information
            </Button>
            <Button disabled={pending()} onClick={() => void showSaveDialog()} style={buttonStyle}>
              Save warning
            </Button>
            <Button disabled={pending()} onClick={() => void showCriticalDialog()} style={buttonStyle}>
              Critical
            </Button>
          </View>

          <View
            style={{
              display: "flex",
              minHeight: 52,
              alignItems: "center",
              justifyContent: "center",
              paddingLeft: 14,
              paddingRight: 14,
              backgroundColor: "#0f131a",
              borderColor: "#252c38",
              borderWidth: 1,
              borderRadius: 9,
            }}
          >
            <Text style={{ color: pending() ? "#c7d2fe" : "#aeb9c9", fontSize: 13 }}>
              {status()}
            </Text>
          </View>
        </View>
      </View>
    </View>
  );
}

render(() => <AlertDialogExample />, mainWindow);
await app.run();
