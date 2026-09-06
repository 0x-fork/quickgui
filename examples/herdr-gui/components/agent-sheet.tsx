import { Button, Text, TextArea, View } from "@quickgui/ui";
import { For, Show } from "@quickgui/ui";

import type { HerdrModel } from "../model.ts";
import { launcherButton } from "../theme.ts";
import { Icon } from "./icon.tsx";

export function AgentSheet(props: { model: HerdrModel }) {
  const model = props.model;
  const theme = model.theme;
  const styles = model.styles;

  return (
    <Show when={model.agentSheetOpen()}>
      <View
        overlay
        focusTrap
        restorePreviousFocus
        style={styles().modalScrim}
      >
        <View
          role="dialog"
          aria-label="New agent"
          aria-modal
          dismissOnEscape
          dismissOnPointerOutside
          style={styles().agentSheet}
          onDismiss={model.closeAgentSheet}
        >
          <View style={styles().sheetHeader}>
            <View style={{ display: "flex", flexDirection: "column", gap: 4 }}>
              <Text
                style={{ color: theme().text, fontSize: 17, fontWeight: 720 }}
              >
                New agent
              </Text>
              <Text style={{ color: theme().textTertiary, fontSize: 12 }}>
                Start a real CLI in {model.activeSpace().name}
              </Text>
            </View>
            <View style={{ flex: 1 }} />
            <Button style={styles().sheetClose} onClick={model.closeAgentSheet}>
              <Icon name="close" size={15} />
            </Button>
          </View>

          <View style={styles().formGroup}>
            <Text style={styles().formLabel}>Agent</Text>
            <View style={{ display: "flex", gap: 8 }}>
              <For each={model.launchers()}>
                {(launcher) => (
                  <Button
                    disabled={!launcher.installed}
                    style={launcherButton(
                      launcher.id === model.selectedLauncherId(),
                      launcher.installed,
                      theme(),
                    )}
                    onClick={() => model.selectLauncher(launcher.id)}
                  >
                    <View style={styles().launcherMark}>
                      <Text style={{ fontSize: 13, fontWeight: 750 }}>
                        {launcher.mark}
                      </Text>
                    </View>
                    <View
                      style={{
                        display: "flex",
                        flexDirection: "column",
                        gap: 2,
                      }}
                    >
                      <Text style={{ fontSize: 12, fontWeight: 620 }}>
                        {launcher.label}
                      </Text>
                      <Text style={{ color: theme().textGhost, fontSize: 10 }}>
                        {launcher.installed ? "Available" : "Not found"}
                      </Text>
                    </View>
                  </Button>
                )}
              </For>
            </View>
            <Text style={{ color: theme().textGhost, fontSize: 11 }}>
              {model.catalogLoading()
                ? "Reading your interactive login-shell PATH…"
                : model.selectedLauncher().description}
            </Text>
          </View>

          <View style={styles().formGroup}>
            <Text style={styles().formLabel}>
              Initial instruction · optional
            </Text>
            <TextArea
              autoFocus
              value={model.initialPrompt()}
              placeholder="What should this agent work on?"
              onInput={(event) => model.setInitialPrompt(event.value ?? "")}
              style={styles().prompt}
            />
          </View>

          <Show
            when={
              !model.catalogLoading() &&
              !model.launchers().some((launcher) => launcher.installed)
            }
          >
            <View style={styles().catalogNotice}>
              <Text
                style={{ color: theme().warning, fontSize: 11.5, lineHeight: 16 }}
              >
                No supported agent CLI was found. You can still open a terminal
                and run any installed agent; the sidebar detects it
                automatically.
              </Text>
            </View>
          </Show>

          <View style={styles().sheetFooter}>
            <Button
              style={styles().toolbarSecondary}
              onClick={model.closeAgentSheet}
            >
              Cancel
            </Button>
            <Button
              disabled={
                model.catalogLoading() || !model.selectedLauncher().installed
              }
              style={{
                ...styles().toolbarPrimary,
                opacity:
                  model.catalogLoading() || !model.selectedLauncher().installed
                    ? 0.45
                    : 1,
              }}
              onClick={model.launchAgent}
            >
              Start {model.selectedLauncher().label}
            </Button>
          </View>
        </View>
      </View>
    </Show>
  );
}
