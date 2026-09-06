import { join } from "node:path";
import { app, Dialog, Window } from "@quickgui/native";
import { Button, Text, View, createRenderer } from "@quickgui/ui";
import { createSignal } from "@quickgui/ui";

await app.whenReady();

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

function FileDialogExample() {
  const window = Window.getCurrentWindow();
  const [pending, setPending] = createSignal(false);
  const [status, setStatus] = createSignal("Choose an open or save dialog.");

  async function openFiles() {
    await present(async () => {
      const result = await Dialog.showOpenDialog({
        title: "Open text files",
        defaultPath: process.cwd(),
        filters: [
          { name: "Text", extensions: ["txt", "md"] },
          { name: "All files", extensions: ["*"] },
        ],
        properties: ["openFile", "multiSelections"],
      }, window);
      return result.canceled
        ? "Open dialog canceled."
        : `Selected ${result.filePaths.length}: ${result.filePaths.join(", ")}`;
    });
  }

  async function openFolder() {
    await present(async () => {
      const result = await Dialog.showOpenDialog({
        title: "Choose a folder",
        defaultPath: process.cwd(),
        properties: ["openDirectory"],
      });
      return result.canceled
        ? "Folder dialog canceled."
        : `Selected folder: ${result.filePaths[0]}`;
    });
  }

  async function saveFile() {
    await present(async () => {
      const result = await Dialog.showSaveDialog({
        title: "Choose a save destination",
        defaultPath: join(process.cwd(), "quickgui-example.txt"),
        filters: [{ name: "Text", extensions: ["txt"] }],
      }, window);
      return result.canceled
        ? "Save dialog canceled."
        : `Save destination: ${result.filePath} (no file was written)`;
    });
  }

  async function present(action: () => Promise<string>) {
    if (pending()) return;
    setPending(true);
    setStatus("Waiting for the native file dialog…");
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
        <Text style={{ fontSize: 14, fontWeight: 600 }}>Native file dialogs</Text>
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
            width: 520,
            gap: 20,
            padding: 28,
            backgroundColor: "#151922",
            borderColor: "#2c3442",
            borderWidth: 1,
            borderRadius: 16,
          }}
        >
          <View style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <Text style={{ fontSize: 26, lineHeight: 32, fontWeight: 700 }}>Open and save</Text>
            <Text style={{ color: "#9aa6b7", fontSize: 14, lineHeight: 21 }}>
              Electron-shaped results backed by native system file dialogs.
            </Text>
          </View>

          <View style={{ display: "flex", gap: 10 }}>
            <Button disabled={pending()} onClick={() => void openFiles()} style={buttonStyle}>
              Open files
            </Button>
            <Button disabled={pending()} onClick={() => void openFolder()} style={buttonStyle}>
              Open folder
            </Button>
            <Button disabled={pending()} onClick={() => void saveFile()} style={buttonStyle}>
              Save file
            </Button>
          </View>

          <View
            style={{
              display: "flex",
              minHeight: 64,
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
            <Text
              style={{
                color: pending() ? "#c7d2fe" : "#aeb9c9",
                fontSize: 13,
                lineHeight: 19,
                textAlign: "center",
              }}
            >
              {status()}
            </Text>
          </View>
        </View>
      </View>
    </View>
  );
}

function openMainWindow() {
  new Window({
    title: "QuickGUI File Dialogs",
    width: 720,
    height: 520,
    minimumWidth: 560,
    minimumHeight: 440,
    background: "#0b0e14",
    renderer: createRenderer(() => <FileDialogExample />),
  });
}

app.onReopen((event) => {
  if (!event.hasVisibleWindows) openMainWindow();
});
openMainWindow();
