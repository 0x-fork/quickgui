import { Button, Text, View } from "@quickgui/ui";
import { For, Show } from "@quickgui/ui";

import {
  agentLabel,
  shortPath,
  tabTitle,
  type AgentStatus,
  type HerdrModel,
  type Pane,
  type Space,
} from "../model.ts";
import { agentRow, sidebarRow } from "../theme.ts";
import { Icon } from "./icon.tsx";
import { SectionHeader, StatusGlyph, statusLabel } from "./status.tsx";

export function Sidebar(props: { model: HerdrModel }) {
  const model = props.model;
  const theme = model.theme;
  const styles = model.styles;

  return (
    <>
      <View style={{ ...styles().sidebar, width: model.sidebarWidth() }}>
        <View style={styles().sidebarTitlebar}>
          <View style={{ flex: 1 }} />
          <Button
            aria-label={
              model.appearance() === "dark"
                ? "Use light appearance"
                : "Use dark appearance"
            }
            focusOnPointer={false}
            style={styles().titlebarIcon}
            onClick={model.toggleTheme}
          >
            <Icon
              name={model.appearance() === "dark" ? "sun" : "moon"}
              size={14}
            />
          </Button>
        </View>

        <View
          style={{
            ...styles().sidebarSection,
            flexGrow: model.sidebarSectionRatio(),
          }}
        >
          <SectionHeader
            label="Spaces"
            action="plus"
            actionLabel="Add space"
            theme={theme()}
            onAction={() => void model.addSpace()}
          />
          <View style={styles().sidebarList}>
            <For each={model.spaces()}>
              {(space) => (
                <SpaceRow
                  space={space}
                  selected={space.id === model.activeSpaceId()}
                  model={model}
                />
              )}
            </For>
          </View>
        </View>

        <View
          aria-label="Resize sidebar sections"
          style={styles().sidebarSectionDivider}
          onPointer={model.handleSidebarSectionPointer}
        >
          <View style={styles().sidebarSectionDividerLine} />
        </View>

        <View
          style={{
            ...styles().sidebarSection,
            flexGrow: 1 - model.sidebarSectionRatio(),
          }}
        >
          <SectionHeader label="Agents" trailing="grouped" theme={theme()} />
          <View style={styles().sidebarList}>
            <Show
              when={model.visibleAgents().length > 0}
              fallback={
                <View style={styles().emptySidebarSection}>
                  <Text style={{ color: theme().textGhost, fontSize: 12 }}>
                    Agents appear here when detected in a pane.
                  </Text>
                </View>
              }
            >
              <For each={model.visibleAgents()}>
                {(pane) => <AgentRow pane={pane} model={model} />}
              </For>
            </Show>
          </View>
        </View>
      </View>

      <View
        aria-label="Resize sidebar"
        style={styles().sidebarDivider}
        onPointer={model.handleSidebarPointer}
      >
        <View style={styles().sidebarDividerLine} />
      </View>
    </>
  );
}

function SpaceRow(props: {
  space: Space;
  selected: boolean;
  model: HerdrModel;
}) {
  const agents = () =>
    props.model
      .panes()
      .filter(
        (pane) =>
          pane.spaceId === props.space.id && pane.status().agent !== undefined,
      );
  const aggregate = () => aggregateStatus(agents());

  return (
    <View style={stylesForSpaceRow(props.selected, props.model)}>
      <Button
        aria-label={"Open " + props.space.name + " space"}
        focusOnPointer={false}
        style={sidebarRow(props.selected, props.model.theme())}
        onClick={() => props.model.selectSpace(props.space)}
      >
        <StatusGlyph status={aggregate()} theme={props.model.theme()} compact />
        <View style={props.model.styles().twoLineRow}>
          <View style={props.model.styles().rowLine}>
            <Text style={props.model.styles().rowTitle}>
              {props.space.name}
            </Text>
            <View style={{ flex: 1 }} />
            <Show when={agents().length > 0}>
              <Text style={props.model.styles().rowCount}>{agents().length}</Text>
            </Show>
          </View>
          <Text style={props.model.styles().rowMeta}>
            {shortPath(props.space.path, props.model.homePath)}
          </Text>
        </View>
      </Button>
      <Show when={props.selected && props.model.spaces().length > 1}>
        <Button
          aria-label={"Remove " + props.space.name + " space"}
          focusOnPointer={false}
          style={props.model.styles().rowClose}
          onClick={() => props.model.removeSpace(props.space)}
        >
          <Icon name="close" size={13} />
        </Button>
      </Show>
    </View>
  );
}

function AgentRow(props: { pane: Pane; model: HerdrModel }) {
  const tab = () =>
    props.model.tabs().find((candidate) => candidate.id === props.pane.tabId);
  const space = () =>
    props.model.spaces().find((candidate) => candidate.id === props.pane.spaceId);
  const title = () => {
    const candidate = tab();
    return candidate ? tabTitle(candidate, props.model.panes()) : "Terminal";
  };

  return (
    <Button
      aria-label={"Open " + agentLabel(props.pane.status().agent)}
      focusOnPointer={false}
      style={agentRow(
        props.pane.id === props.model.activePaneId(),
        props.model.theme(),
      )}
      onClick={() => props.model.selectPane(props.pane)}
    >
      <StatusGlyph
        status={props.pane.status().agentStatus}
        theme={props.model.theme()}
      />
      <View style={props.model.styles().twoLineRow}>
        <View style={props.model.styles().rowLine}>
          <Text style={props.model.styles().agentTitle}>{space()?.name}</Text>
          <Text style={props.model.styles().rowSeparator}>·</Text>
          <Text style={props.model.styles().rowMeta}>{title()}</Text>
        </View>
        <View style={props.model.styles().rowLine}>
          <Text style={props.model.styles().rowMeta}>
            {agentLabel(props.pane.status().agent)}
          </Text>
          <View style={{ flex: 1 }} />
          <Text
            style={{
              color:
                props.pane.status().agentStatus === "blocked"
                  ? props.model.theme().danger
                  : props.model.theme().textGhost,
              fontSize: 11.5,
              lineHeight: 15,
            }}
          >
            {statusLabel(props.pane.status().agentStatus)}
          </Text>
        </View>
      </View>
    </Button>
  );
}

function aggregateStatus(panes: readonly Pane[]): AgentStatus | undefined {
  if (panes.some((pane) => pane.status().agentStatus === "blocked")) {
    return "blocked";
  }
  if (panes.some((pane) => pane.status().agentStatus === "working")) {
    return "working";
  }
  if (panes.some((pane) => pane.status().agentStatus === "idle")) {
    return "idle";
  }
  return undefined;
}

function stylesForSpaceRow(selected: boolean, model: HerdrModel) {
  return {
    position: "relative" as const,
    display: "flex" as const,
    flexShrink: 0,
    height: 48,
    borderRadius: 6,
    backgroundColor: selected ? model.theme().selected : "transparent",
  };
}
