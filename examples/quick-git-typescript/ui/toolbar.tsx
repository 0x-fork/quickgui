import { Button, Progress, Text, View } from "@quickgui/solid";
import { Show } from "solid-js";
import { useApp } from "./context.tsx";
import { Icon, type IconName } from "./icons.tsx";

export function Toolbar() {
  const app = useApp();
  const store = app.store;
  const busy = () => store.busy() !== undefined;
  return (
    <View style={app.styles().toolbar}>
      <Button
        onClick={() => store.setView("branches")}
        style={[
          app.styles().button("secondary"),
          {
            flexDirection: "row",
            height: 28,
            maxWidth: 260,
            bg: "transparent",
            borderWidth: 0,
          },
        ]}
      >
        <Icon name="branch" size={14} color={app.theme().textSecondary} />
        <Text style={{ minWidth: 0, lineClamp: 1 }}>
          {store.status()?.branch ||
            (store.status()?.detached
              ? `${store.status()?.headSha?.slice(0, 7) ?? "HEAD"} (detached)`
              : "…")}
        </Text>
      </Button>
      <Show
        when={
          store.status()?.hasUpstreamCounts &&
          ((store.status()?.ahead ?? 0) > 0 || (store.status()?.behind ?? 0) > 0)
        }
      >
        <View
          aria-label="Upstream commit counts"
          style={{
            display: "flex",
            alignItems: "center",
            gap: 3,
            fontSize: 11,
            color: app.theme().textSecondary,
            appRegion: "no-drag",
          }}
        >
          <Icon name="arrow-up" size={14} />
          <Text>{store.status()?.ahead ?? 0}</Text>
          <Icon name="arrow-down" size={14} />
          <Text>{store.status()?.behind ?? 0}</Text>
        </View>
      </Show>
      <Show when={store.conflicts() > 0}>
        <Text
          style={{
            fontSize: 11,
            fontWeight: 600,
            color: app.theme().warning,
            appRegion: "no-drag",
          }}
        >
          {`${store.conflicts()} conflicted`}
        </Text>
      </Show>
      <View style={{ flex: 1, appRegion: "drag" }} />
      <Show when={busy()}>
        <View
          style={{
            display: "flex",
            flexDirection: "row",
            alignItems: "center",
            gap: 8,
            marginRight: 8,
            appRegion: "no-drag",
          }}
        >
          <Progress.Root
            indeterminate
            aria-label="Git operation in progress"
            style={{ width: 32, height: 4, flexShrink: 0 }}
          >
            <Progress.Track
              style={{
                width: "100%",
                height: 4,
                borderRadius: 2,
                bg: app.theme().borderStrong,
              }}
            >
              <Progress.Indicator
                style={{
                  width: 12,
                  height: 4,
                  borderRadius: 2,
                  bg: app.theme().textSecondary,
                }}
              />
            </Progress.Track>
          </Progress.Root>
          <Text
            style={{ fontSize: 12, color: app.theme().textSecondary }}
          >{`${store.busy()?.label ?? ""}…`}</Text>
          <Show when={store.busy()?.cancel}>
            <Button onClick={() => store.cancelBusy()} style={app.styles().button("secondary")}>
              Cancel
            </Button>
          </Show>
        </View>
      </Show>
      <ToolbarAction
        label="Fetch"
        icon="cloud-down"
        disabled={busy()}
        onClick={() => void store.fetch()}
      />
      <ToolbarAction
        label="Pull"
        icon="arrow-down"
        disabled={busy() || !store.status()?.upstream}
        onClick={() => void store.pull()}
      />
      <ToolbarAction
        label="Push"
        icon="arrow-up"
        disabled={busy() || !store.status()?.branch}
        onClick={() => void store.push()}
      />
      <ToolbarAction
        label="Refresh"
        icon="refresh"
        iconOnly
        disabled={busy()}
        onClick={() => void store.refresh()}
      />
    </View>
  );
}

function ToolbarAction(props: {
  label: string;
  icon: IconName;
  disabled: boolean;
  iconOnly?: boolean;
  onClick: () => void;
}) {
  const app = useApp();
  return (
    <Button
      aria-label={props.label}
      disabled={props.disabled}
      onClick={props.onClick}
      style={[
        app.styles().button("secondary"),
        { flexDirection: "row", height: 28, borderRadius: 7 },
      ]}
    >
      <Icon name={props.icon} size={14} color={app.theme().textSecondary} />
      <Show when={!props.iconOnly}>
        <Text>{props.label}</Text>
      </Show>
    </Button>
  );
}
