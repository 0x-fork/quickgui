import { Button, Text, View, type JSX } from "@quickgui/solid";
import { Show, type Element as SolidElement } from "solid-js";

import { Icon, type IconName } from "./icons.tsx";
import type { Styles, Theme } from "./theme.ts";

export interface UiContext {
  theme: () => Theme;
  styles: () => Styles;
}

export function ToolbarButton(props: {
  ui: UiContext;
  icon: IconName;
  label?: string | undefined;
  tooltip?: string | undefined;
  disabled?: boolean | undefined;
  onClick: () => void;
  style?: JSX.StyleProp;
}) {
  return (
    <Button
      aria-label={props.tooltip ?? props.label ?? props.icon}
      {...(props.tooltip ? { tooltip: props.tooltip, tooltipPlacement: "bottom" as const, tooltipDelay: 500 } : {})}
      disabled={props.disabled ?? false}
      focusOnPointer={false}
      onClick={props.onClick}
      style={[
        {
          display: "flex",
          height: 28,
          flexShrink: 0,
          alignItems: "center",
          justifyContent: "center",
          gap: 6,
          paddingLeft: props.label ? 9 : 0,
          paddingRight: props.label ? 10 : 0,
          ...(props.label ? {} : { width: 28 }),
          borderRadius: 7,
          backgroundColor: "transparent",
          color: props.ui.theme().textSecondary,
          fontSize: 12.5,
          fontWeight: 600,
          cursor: "default",
          userSelect: "none",
          appRegion: "no-drag",
          transition: "background-color 90ms, color 90ms, opacity 90ms",
          hover: { backgroundColor: props.ui.theme().hover, color: props.ui.theme().text },
          active: { backgroundColor: props.ui.theme().active },
          focus: { outline: `2px solid ${props.ui.theme().focusRing}` },
          disabled: { opacity: 0.4 },
        },
        props.style,
      ]}
    >
      <Icon name={props.icon} size={15} />
      <Show when={props.label}>
        <Text>{props.label}</Text>
      </Show>
    </Button>
  );
}

export function IconButton(props: {
  ui: UiContext;
  icon: IconName;
  label: string;
  size?: number;
  iconSize?: number;
  disabled?: boolean;
  onClick: () => void;
  style?: JSX.StyleProp;
}) {
  return (
    <Button
      aria-label={props.label}
      tooltip={props.label}
      tooltipDelay={600}
      disabled={props.disabled ?? false}
      focusOnPointer={false}
      onClick={props.onClick}
      style={[props.ui.styles().iconButton(props.size ?? 24), props.style]}
    >
      <Icon name={props.icon} size={props.iconSize ?? 14} />
    </Button>
  );
}

export function PushButton(props: {
  ui: UiContext;
  kind?: "primary" | "secondary" | "danger";
  icon?: IconName;
  label: string;
  disabled?: boolean;
  onClick: () => void;
  style?: JSX.StyleProp;
  keymap?: Readonly<Record<string, string>>;
}) {
  return (
    <Button
      aria-label={props.label}
      disabled={props.disabled ?? false}
      onClick={props.onClick}
      style={[props.ui.styles().button(props.kind ?? "secondary"), props.style]}
    >
      <Show when={props.icon}>{(icon) => <Icon name={icon()} size={14} />}</Show>
      <Text>{props.label}</Text>
    </Button>
  );
}

export function Badge(props: { ui: UiContext; children: SolidElement; color?: string; background?: string }) {
  return (
    <View
      style={[
        props.ui.styles().badge,
        props.color ? { color: props.color } : null,
        props.background ? { backgroundColor: props.background } : null,
      ]}
    >
      <Text>{props.children}</Text>
    </View>
  );
}

export function EmptyState(props: {
  ui: UiContext;
  icon: IconName;
  title: string;
  description?: string | undefined;
  children?: SolidElement;
}) {
  return (
    <View style={props.ui.styles().emptyState}>
      <Text style={props.ui.styles().emptyTitle}>{props.title}</Text>
      <Show when={props.description}>
        <Text style={props.ui.styles().emptyCopy}>{props.description}</Text>
      </Show>
      {props.children}
    </View>
  );
}

export function SectionHeader(props: {
  ui: UiContext;
  label: string;
  count?: number;
  action?: { icon: IconName; label: string; onClick: () => void; disabled?: boolean };
}) {
  return (
    <View style={props.ui.styles().sectionLabel}>
      <Text style={props.ui.styles().sectionLabelText}>{props.label}</Text>
      <Show when={props.count !== undefined && props.count > 0}>
        <Text style={{ color: props.ui.theme().textTertiary, fontSize: 11, fontWeight: 600 }}>{props.count}</Text>
      </Show>
      <Show when={props.action}>
        {(action) => (
          <IconButton
            ui={props.ui}
            icon={action().icon}
            label={action().label}
            size={20}
            iconSize={12}
            disabled={action().disabled ?? false}
            onClick={action().onClick}
          />
        )}
      </Show>
    </View>
  );
}

/** A tiny inline spinner substitute: three dots that the core animates through opacity. */
export function Working(props: { ui: UiContext; label: string }) {
  return (
    <View style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 6 }}>
      <View
        style={{
          width: 6,
          height: 6,
          borderRadius: 3,
          backgroundColor: props.ui.theme().accent,
        }}
      />
      <Text style={{ fontSize: 12, color: props.ui.theme().textSecondary }}>{props.label}</Text>
    </View>
  );
}
