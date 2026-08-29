import {
  app,
  AutoStart,
  Clipboard,
  DeepLink,
  GlobalShortcut,
  Menu,
  Notifications,
  PowerMonitor,
  Screen,
  SecureStorage,
  Shell,
  Tray,
  type TrayIcon,
  Updater,
  Window,
} from "@quickgui/native";
import { Button, Text, View, createRenderer } from "@quickgui/solid";
import { createSignal } from "solid-js";

const identifier = "dev.quickgui.system-api-example";
const appName = "QuickGUI System APIs";
await app.whenReady();

const autoStartOptions = {
  appName: identifier,
  bundleIdentifier: identifier,
} as const;
const protocolOptions = {
  scheme: "quickgui-system",
  appName,
  appId: identifier,
} as const;

const buttonStyle = {
  display: "flex",
  height: 38,
  alignItems: "center",
  justifyContent: "center",
  paddingLeft: 14,
  paddingRight: 14,
  backgroundColor: "#252b37",
  color: "#f4f7fb",
  borderColor: "#394253",
  borderWidth: 1,
  borderRadius: 8,
  cursor: "default",
  userSelect: "none",
} as const;

function SystemApiExample() {
  const window = Window.getCurrentWindow();
  const [status, setStatus] = createSignal(
    `Primary instance · updater target ${Updater.defaultTarget()}`,
  );
  const [busy, setBusy] = createSignal(false);
  let tray: TrayIcon | undefined;
  let unregisterShortcut: (() => Promise<void>) | undefined;

  app.on("secondInstance", ({ argv }) => {
    window.show();
    window.focus();
    setStatus(`A second launch forwarded ${argv.length} argument(s).`);
  });
  app.on("openUrls", (urls) => setStatus(`Deep link: ${urls.join(", ")}`));
  PowerMonitor.on("suspend", () => setStatus("The system is suspending."));
  PowerMonitor.on("resume", () => setStatus("The system resumed."));
  PowerMonitor.on("lock-screen", () => setStatus("The session was locked."));
  PowerMonitor.on("unlock-screen", () => setStatus("The session was unlocked."));
  Notifications.onResponse(({ tag, actionId }) => {
    setStatus(`Notification ${tag} activated${actionId ? ` via ${actionId}` : ""}.`);
  });

  Menu.setApplicationMenu([
    {
      label: "App",
      items: [
        { label: "Show window", click: () => window.show() },
        { type: "separator" },
        { label: "Quit", click: () => app.quit() },
      ],
    },
  ]);

  async function perform(label: string, operation: () => Promise<string>): Promise<void> {
    if (busy()) return;
    setBusy(true);
    setStatus(`${label}…`);
    try {
      setStatus(await operation());
    } catch (error) {
      setStatus(`${label} failed: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setBusy(false);
    }
  }

  const action = (label: string, operation: () => Promise<string>) => (
    <Button disabled={busy()} style={buttonStyle} onClick={() => void perform(label, operation)}>
      {label}
    </Button>
  );

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
        <Text style={{ fontSize: 14, fontWeight: 600 }}>Core-first system APIs</Text>
      </View>

      <View
        style={{
          display: "flex",
          flexDirection: "column",
          flex: 1,
          minHeight: 0,
          gap: 18,
          padding: 28,
        }}
      >
        <View style={{ display: "flex", flexDirection: "column", gap: 7 }}>
          <Text style={{ fontSize: 26, lineHeight: 32, fontWeight: 700 }}>Native integrations</Text>
          <Text style={{ color: "#9aa6b7", fontSize: 14, lineHeight: 21 }}>
            Every button calls a Rust core capability re-exported by the Solid binding.
          </Text>
        </View>

        <View style={{ display: "flex", flexWrap: "wrap", gap: 10 }}>
          {action("Clipboard", async () => {
            await Clipboard.writeText("Hello from QuickGUI");
            return `Clipboard: ${await Clipboard.readText()}`;
          })}
          {action("Displays", async () => {
            const displays = Screen.getAllDisplays();
            return `${displays.length} display(s), primary ${Screen.getPrimaryDisplay()?.name ?? "unknown"}`;
          })}
          {action("Notification", async () => {
            await Notifications.show({
              tag: "system-api-example",
              title: "QuickGUI",
              body: "Native notification delivery is working.",
              actions: [{ id: "open", label: "Open" }],
            });
            return "Notification delivered.";
          })}
          {action("Open website", async () => {
            await Shell.openExternal("https://github.com/egoist/quickgui");
            return "Opened the QuickGUI repository.";
          })}
          {action("Secure storage", async () => {
            await SecureStorage.setText(identifier, "example-token", "native-secret");
            return `Credential round trip: ${await SecureStorage.getText(identifier, "example-token")}`;
          })}
          {action("Toggle autostart", async () => {
            const enabled = await AutoStart.isEnabled(autoStartOptions);
            if (enabled) await AutoStart.disable(autoStartOptions);
            else await AutoStart.enable(autoStartOptions);
            return `Autostart is now ${enabled ? "disabled" : "enabled"}.`;
          })}
          {action("Register deep link", async () => {
            if (!DeepLink.supportsDynamicRegistration()) {
              return "macOS deep links are declared by `protocols` in quickgui.config.ts.";
            }
            await DeepLink.register(protocolOptions);
            return "Registered quickgui-system:// with the current user account.";
          })}
          {action("Global shortcut", async () => {
            if (unregisterShortcut) {
              await unregisterShortcut();
              unregisterShortcut = undefined;
              return "Unregistered shift+alt+KeyQ.";
            }
            unregisterShortcut = await GlobalShortcut.register("shift+alt+KeyQ", () => {
              window.show();
              window.focus();
              setStatus("The global shortcut was pressed.");
            });
            return "Registered shift+alt+KeyQ.";
          })}
          {action("Tray icon", async () => {
            if (tray) {
              await tray.destroy();
              tray = undefined;
              return "Removed the tray icon.";
            }
            tray = await Tray.create({
              icon: makeTrayIcon(),
              tooltip: appName,
              menu: [
                { label: "Show window", click: () => window.show() },
                { type: "separator" },
                { label: "Quit", click: () => app.quit() },
              ],
            });
            tray.on("click", () => setStatus("Tray icon clicked."));
            return "Created the tray icon.";
          })}
        </View>

        <View
          style={{
            display: "flex",
            minHeight: 68,
            alignItems: "center",
            justifyContent: "center",
            paddingLeft: 16,
            paddingRight: 16,
            backgroundColor: "#111620",
            borderColor: "#293242",
            borderWidth: 1,
            borderRadius: 10,
          }}
        >
          <Text style={{ color: busy() ? "#c7d2fe" : "#aeb9c9", fontSize: 13, lineHeight: 19 }}>
            {status()}
          </Text>
        </View>
      </View>
    </View>
  );
}

function makeTrayIcon(): { data: Uint8Array; width: number; height: number } {
  const width = 20;
  const height = 20;
  const data = new Uint8Array(width * height * 4);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const offset = (y * width + x) * 4;
      const inside = (x - 9.5) ** 2 + (y - 9.5) ** 2 < 78;
      data[offset] = inside ? 122 : 0;
      data[offset + 1] = inside ? 162 : 0;
      data[offset + 2] = inside ? 247 : 0;
      data[offset + 3] = inside ? 255 : 0;
    }
  }
  return { data, width, height };
}

const primary = await app.requestSingleInstanceLock(identifier);
if (!primary) {
  app.destroy();
  process.exit(0);
}

new Window({
  title: appName,
  width: 760,
  height: 640,
  minimumWidth: 620,
  minimumHeight: 520,
  background: "#0b0e14",
  renderer: createRenderer(() => <SystemApiExample />),
});
