import { Text, View } from "@quickgui/solid";
import { Button as SwiftButton, Host, ProgressView } from "@quickgui/solid/swift-ui";
import { buttonStyle, controlSize, disabled } from "@quickgui/solid/swift-ui/modifiers";
import { Show } from "solid-js";

import { useApp } from "./context.tsx";
import { Icon } from "./icons.tsx";

/**
 * The window toolbar. Its actions are real SwiftUI buttons — AppKit paints them in the current
 * system style with SF Symbols, hover, and focus — while the branch readout stays a QuickGUI
 * element because it carries custom counters.
 */
export function Toolbar() {
  const app = useApp();
  const store = app.store;
  const status = () => store.status();
  const busy = () => store.busy() !== undefined;

  return (
    <View style={app.styles().toolbar}>
      <View style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 8, minWidth: 0, appRegion: "no-drag" }}>
        <Host matchContents>
          <SwiftButton
            label={status()?.branch ?? (status()?.detached ? `${status()?.headSha?.slice(0, 7) ?? "HEAD"} (detached)` : "…")}
            systemImage="arrow.triangle.branch"
            modifiers={[buttonStyle("borderless"), controlSize("regular")]}
            onPress={() => store.setView("branches")}
          />
        </Host>
        <Show when={status()?.hasUpstreamCounts && ((status()?.ahead ?? 0) > 0 || (status()?.behind ?? 0) > 0)}>
          <View style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 4 }}>
            <Show when={(status()?.ahead ?? 0) > 0}>
              <Counter icon="arrow-up" value={status()!.ahead} />
            </Show>
            <Show when={(status()?.behind ?? 0) > 0}>
              <Counter icon="arrow-down" value={status()!.behind} />
            </Show>
          </View>
        </Show>
        <Show when={store.conflicts() > 0}>
          <View style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 4, height: 20, paddingLeft: 7, paddingRight: 8, borderRadius: 5, backgroundColor: app.theme().warningWash, color: app.theme().warning }}>
            <Icon name="alert" size={12} />
            <Text style={{ fontSize: 11, fontWeight: 600 }}>{`${store.conflicts()} conflicted`}</Text>
          </View>
        </Show>
      </View>
      <View style={{ flex: 1, appRegion: "drag" }} />
      <Show when={store.busy()}>
        {(state) => (
          <View style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 8, marginRight: 8, appRegion: "no-drag" }}>
            <Host matchContents>
              <ProgressView modifiers={[controlSize("small")]} />
            </Host>
            <Text style={{ fontSize: 12, color: app.theme().textSecondary }}>{`${state().label}…`}</Text>
            <Show when={state().cancel}>
              <Host matchContents>
                <SwiftButton label="Cancel" modifiers={[buttonStyle("borderless"), controlSize("small")]} onPress={() => store.cancelBusy()} />
              </Host>
            </Show>
          </View>
        )}
      </Show>
      <View style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 6, appRegion: "no-drag" }}>
        <Host matchContents>
          <SwiftButton label="Fetch" systemImage="arrow.down.to.line" modifiers={[buttonStyle("glass"), disabled(busy())]} onPress={() => void store.fetch()} />
        </Host>
        <Host matchContents>
          <SwiftButton label="Pull" systemImage="arrow.down" modifiers={[buttonStyle("glass"), disabled(busy() || !status()?.upstream)]} onPress={() => void store.pull()} />
        </Host>
        <Host matchContents>
          <SwiftButton label="Push" systemImage="arrow.up" modifiers={[buttonStyle("glass"), disabled(busy() || !status()?.branch)]} onPress={() => void store.push()} />
        </Host>
        <Host matchContents>
          <SwiftButton label="" systemImage="arrow.clockwise" modifiers={[buttonStyle("glass"), disabled(busy())]} onPress={() => void store.refresh()} />
        </Host>
      </View>
    </View>
  );
}

function Counter(props: { icon: "arrow-up" | "arrow-down"; value: number }) {
  const app = useApp();
  return (
    <View style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 2, height: 18, paddingLeft: 4, paddingRight: 6, borderRadius: 4, backgroundColor: app.theme().hover, color: app.theme().textSecondary }}>
      <Icon name={props.icon} size={11} />
      <Text style={{ fontSize: 11, fontWeight: 600 }}>{String(props.value)}</Text>
    </View>
  );
}
