import { Button, Text, View } from "@quickgui/ui";
import { Show } from "@quickgui/ui";

import type { AgentStatus } from "../model.ts";
import type { Theme } from "../theme.ts";
import { headingActionStyle } from "../theme.ts";
import { Icon, type IconName } from "./icon.tsx";

export function SectionHeader(props: {
  label: string;
  action?: IconName;
  actionLabel?: string;
  trailing?: string;
  theme: Theme;
  onAction?(): void;
}) {
  return (
    <View
      style={{
        display: "flex",
        height: 30,
        flexShrink: 0,
        alignItems: "center",
        paddingLeft: 10,
        paddingRight: 6,
      }}
    >
      <Text
        style={{
          color: props.theme.textTertiary,
          fontSize: 12,
          fontWeight: 650,
        }}
      >
        {props.label}
      </Text>
      <View style={{ flex: 1 }} />
      <Show when={props.trailing}>
        {(() => { const trailing = () => (props.trailing)!; return (
          <Text style={{ color: props.theme.textGhost, fontSize: 10.5 }}>
            {trailing()}
          </Text>
        ); })()}
      </Show>
      <Show when={props.action}>
        {(() => { const action = () => (props.action)!; return (
          <Button
            aria-label={props.actionLabel ?? props.label}
            focusOnPointer={false}
            style={headingActionStyle(props.theme)}
            onClick={() => { const action = props.onAction; if (action !== undefined) action(); }}
          >
            <Icon name={action()} size={14} />
          </Button>
        ); })()}
      </Show>
    </View>
  );
}

export function StatusGlyph(props: {
  status: AgentStatus | undefined;
  theme: Theme;
  compact?: boolean;
}) {
  const color = () => agentStatusColor(props.status, props.theme);
  const icon = (): IconName => {
    switch (props.status) {
      case "blocked":
        return "warning-circle";
      case "working":
        return "loader";
      case "idle":
        return "check-circle";
      default:
        return "circle";
    }
  };
  return (
    <View
      style={{
        display: "flex",
        width: props.compact ? 12 : 14,
        height: props.compact ? 12 : 14,
        flexShrink: 0,
        alignItems: "center",
        justifyContent: "center",
      }}
    >
      <Icon
        name={icon()}
        size={props.compact ? 11 : 13}
        color={color()}
      />
    </View>
  );
}

export function StatusPill(props: {
  status: AgentStatus | undefined;
  theme: Theme;
}) {
  return (
    <Show when={props.status}>
      {(() => { const status = () => (props.status)!; return (
        <View
          style={{
            display: "flex",
            height: 20,
            alignItems: "center",
            gap: 5,
            paddingLeft: 7,
            paddingRight: 7,
            backgroundColor:
              status() === "blocked"
                ? props.theme.warningWash
                : status() === "working"
                  ? props.theme.workingWash
                  : props.theme.successWash,
            borderRadius: 10,
          }}
        >
          <Text
            style={{
              color: agentStatusColor(status(), props.theme),
              fontSize: 10.5,
              fontWeight: 600,
            }}
          >
            {statusLabel(status())}
          </Text>
        </View>
      ); })()}
    </Show>
  );
}

export function statusLabel(status: AgentStatus | undefined): string {
  switch (status) {
    case "blocked":
      return "needs input";
    case "working":
      return "working";
    case "idle":
      return "idle";
    default:
      return "";
  }
}

export function agentStatusColor(
  status: AgentStatus | undefined,
  theme: Theme,
): string {
  switch (status) {
    case "blocked":
      return theme.danger;
    case "working":
      return theme.working;
    case "idle":
      return theme.success;
    default:
      return theme.textGhost;
  }
}
