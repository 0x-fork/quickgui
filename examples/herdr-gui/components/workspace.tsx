import { Button, Terminal, Text, View } from "@quickgui/solid";
import { For, Show } from "solid-js";

import {
  spaceForPane,
  tabTitle,
  type HerdrModel,
  type Pane,
} from "../model.ts";
import { paneStyle, tabButton, tabItem } from "../theme.ts";
import { Icon } from "./icon.tsx";

export function Workspace(props: { model: HerdrModel }) {
  const model = props.model;
  const theme = model.theme;
  const styles = model.styles;

  return (
    <View style={styles().main}>
      <TabBar model={model} />
      <View style={styles().hairline} />

      <View style={styles().workspaceSurface}>
        <For each={model.tabs().map((tab) => tab.id)}>
          {(tabId) => <TabSurface tabId={tabId} model={model} />}
        </For>

        <Show when={!model.activeTab()}>
          <View style={styles().emptyState}>
            <Icon name="terminal" size={24} color={theme().textGhost} />
            <Text style={styles().emptyCopy}>Open a tab to start working</Text>
            <Button style={styles().smallButton} onClick={() => model.newTerminal()}>
              New Tab
            </Button>
          </View>
        </Show>
      </View>

      <Show
        when={
          model.activePane()?.status().status === "failed" &&
          model.activePane()?.status().message
        }
      >
        <View style={styles().errorBar}>
          <Text style={{ color: theme().danger, fontSize: 11.5 }}>
            {model.activePane()?.status().message}
          </Text>
          <View style={{ flex: 1 }} />
          <Show when={model.activePane()}>
            {(pane) => (
              <Button
                style={styles().errorAction}
                onClick={() => model.restartPane(pane())}
              >
                Restart
              </Button>
            )}
          </Show>
        </View>
      </Show>
    </View>
  );
}

function TabBar(props: { model: HerdrModel }) {
  const model = props.model;
  const styles = model.styles;
  const theme = model.theme;

  return (
    <View style={styles().tabBar}>
      <View style={styles().tabStrip}>
        <For each={model.activeSpaceTabs().map((tab) => tab.id)}>
          {(tabId) => {
            const tab = () =>
              model.tabs().find((candidate) => candidate.id === tabId);
            const active = () => tabId === model.activeTabId();
            return (
              <Show when={tab()}>
                {(candidate) => (
                  <View style={tabItem(active(), theme())}>
                    <Button
                      aria-label={"Open " + tabTitle(candidate(), model.panes())}
                      style={tabButton(active(), theme())}
                      onClick={() => model.selectTab(candidate())}
                    >
                      <Text style={styles().tabTitle}>
                        {tabTitle(candidate(), model.panes())}
                      </Text>
                    </Button>
                    <Show when={active()}>
                      <Button
                        aria-label="Close tab"
                        style={styles().tabClose}
                        onClick={() => model.closeTab(tabId)}
                      >
                        <Icon name="close" size={13} />
                      </Button>
                    </Show>
                  </View>
                )}
              </Show>
            );
          }}
        </For>
        <Button
          aria-label="New tab"
          style={styles().tabAdd}
          onClick={() => model.newTerminal()}
        >
          <Icon name="plus" size={14} />
        </Button>
      </View>

      <View style={{ flex: 1, appRegion: "drag" }} />
      <Button
        focusOnPointer={false}
        style={styles().agentAction}
        onClick={model.openAgentSheet}
      >
        <Icon name="plus" size={13} />
        <Text>Agent</Text>
      </Button>
      <View style={styles().toolbarSeparator} />
      <Button
        aria-label="Split right"
        style={styles().toolbarIcon}
        onClick={() => model.splitTerminal("horizontal")}
      >
        <Icon name="columns" size={15} />
      </Button>
      <Button
        aria-label="Split down"
        style={styles().toolbarIcon}
        onClick={() => model.splitTerminal("vertical")}
      >
        <Icon name="rows" size={15} />
      </Button>
    </View>
  );
}

function TabSurface(props: { tabId: number; model: HerdrModel }) {
  const model = props.model;
  const tab = () => model.tabs().find((candidate) => candidate.id === props.tabId);
  const paneList = () => {
    const candidate = tab();
    if (!candidate) return [];
    return candidate.paneIds
      .map((id) => model.panes().find((pane) => pane.id === id))
      .filter((pane): pane is Pane => pane !== undefined);
  };

  return (
    <Show when={tab()}>
      {(candidate) => (
        <View
          style={{
            ...model.styles().tabSurface,
            flexDirection:
              candidate().splitDirection === "horizontal" ? "row" : "column",
            visibility:
              candidate().id === model.activeTabId() ? "visible" : "hidden",
          }}
        >
          <For each={paneList()}>
            {(pane) => <TerminalPane pane={pane} model={model} />}
          </For>
        </View>
      )}
    </Show>
  );
}

function TerminalPane(props: { pane: Pane; model: HerdrModel }) {
  const model = props.model;
  const theme = model.theme;
  const styles = model.styles;
  const space = () => spaceForPane(props.pane, model.spaces());
  return (
    <View style={paneStyle(theme())}>
      <View style={styles().terminalBody}>
        <Terminal
          ref={(node) => model.registerTerminal(props.pane.id, node)}
          program={props.pane.program}
          arguments={props.pane.arguments}
          workingDirectory={space().path}
          {...(props.pane.environment
            ? { environment: props.pane.environment }
            : {})}
          scrollback={50_000}
          terminalPalette={theme().terminalPalette}
          terminalCursorColor={theme().terminalCursor}
          onClick={() => model.selectPane(props.pane)}
          onStatus={(event) => model.handleTerminalStatus(props.pane, event)}
          style={{
            position: "absolute",
            top: 0,
            right: 0,
            bottom: 0,
            left: 0,
            padding: 1,
            backgroundColor: theme().terminal,
            color: theme().terminalText,
            fontFamily: "JetBrainsMono Nerd Font Mono",
            fontSize: 14,
            lineHeight: 20.5,
          }}
        />
      </View>
    </View>
  );
}
