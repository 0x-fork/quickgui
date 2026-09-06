import { Button, Text, View, type Style, type NativeNode } from "@quickgui/ui";
import { Show, type Keymap } from "@quickgui/ui";

import { Icon, type IconName } from "./icons.tsx";
import type { Styles, Theme } from "./theme.ts";

export interface UiContext {
  theme: () => Theme;
  styles: () => Styles;
}

export function ToolbarButton(props: {
  ui: UiContext;
  icon: () => (IconName);
  label?: () => (string | undefined);
  tooltip?: () => (string | undefined);
  disabled?: () => (boolean | undefined);
  onClick: () => void;
  style?: () => (Style);
}) {
  const readlabel = () => { const source = props.label; return source === undefined ? undefined : source(); };
  const readtooltip = () => { const source = props.tooltip; return source === undefined ? undefined : source(); };
  const readdisabled = () => { const source = props.disabled; return source === undefined ? undefined : source(); };
  const readstyle = () => { const source = props.style; return source === undefined ? undefined : source(); };
  return (
    <Button
      aria-label={readtooltip() ?? readlabel() ?? props.icon()}
      tooltip={readtooltip() ?? ""} tooltipPlacement="bottom" tooltipDelay={500}
      disabled={readdisabled() ?? false}
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
          paddingLeft: readlabel() ? 9 : 0,
          paddingRight: readlabel() ? 10 : 0,
          ...(readlabel() ? {} : { width: 28 }),
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
        readstyle() ?? {},
      ]}
    >
      <Icon name={props.icon()} size={15} />
      <Show when={readlabel()}>
        <Text>{readlabel()}</Text>
      </Show>
    </Button>
  );
}

export function IconButton(props: {
  ui: UiContext;
  icon: () => (IconName);
  label: () => (string);
  size?: () => (number);
  iconSize?: () => (number);
  disabled?: () => (boolean);
  onClick: () => void;
  style?: () => (Style);
}) {
  const readsize = () => { const source = props.size; return source === undefined ? undefined : source(); };
  const readiconSize = () => { const source = props.iconSize; return source === undefined ? undefined : source(); };
  const readdisabled = () => { const source = props.disabled; return source === undefined ? undefined : source(); };
  const readstyle = () => { const source = props.style; return source === undefined ? undefined : source(); };
  return (
    <Button
      aria-label={props.label()}
      tooltip={props.label()}
      tooltipDelay={600}
      disabled={readdisabled() ?? false}
      focusOnPointer={false}
      onClick={props.onClick}
      style={[props.ui.styles().iconButton(readsize() ?? 24), readstyle() ?? {}]}
    >
      <Icon name={props.icon()} size={readiconSize() ?? 14} />
    </Button>
  );
}

export function PushButton(props: {
  ui: UiContext;
  kind?: () => ("primary" | "secondary" | "danger");
  icon?: () => (IconName);
  label: () => (string);
  disabled?: () => (boolean);
  onClick: () => void;
  style?: () => (Style);
  keymap?: Keymap;
}) {
  const readkind = () => { const source = props.kind; return source === undefined ? undefined : source(); };
  const readicon = () => { const source = props.icon; return source === undefined ? undefined : source(); };
  const readdisabled = () => { const source = props.disabled; return source === undefined ? undefined : source(); };
  const readstyle = () => { const source = props.style; return source === undefined ? undefined : source(); };
  return (
    <Button
      aria-label={props.label()}
      disabled={readdisabled() ?? false}
      onClick={props.onClick}
      style={[props.ui.styles().button(readkind() ?? "secondary"), readstyle() ?? {}]}
    >
      <Show when={readicon()}>{(() => { const icon = () => (readicon())!; return <Icon name={icon()} size={14} />; })()}</Show>
      <Text>{props.label()}</Text>
    </Button>
  );
}

export function Badge(props: { ui: UiContext; children: () => NativeNode; color?: () => (string); background?: () => (string) }) {
  const readcolor = () => { const source = props.color; return source === undefined ? undefined : source(); };
  const readbackground = () => { const source = props.background; return source === undefined ? undefined : source(); };
  return (
    <View
      style={[
        props.ui.styles().badge,
        readcolor() ? { color: readcolor()! } : {},
        readbackground() ? { backgroundColor: readbackground()! } : {},
      ]}
    >
      <Text>{props.children?.()}</Text>
    </View>
  );
}

export function EmptyState(props: {
  ui: UiContext;
  icon: () => (IconName);
  title: () => (string);
  description?: () => (string | undefined);
  children?: () => NativeNode;
}) {
  const readdescription = () => { const source = props.description; return source === undefined ? undefined : source(); };
  return (
    <View style={props.ui.styles().emptyState}>
      <Text style={props.ui.styles().emptyTitle}>{props.title()}</Text>
      <Show when={readdescription()}>
        <Text style={props.ui.styles().emptyCopy}>{readdescription()}</Text>
      </Show>
      {props.children?.()}
    </View>
  );
}

export function SectionHeader(props: {
  ui: UiContext;
  label: () => (string);
  count?: () => (number);
  action?: { icon: IconName; label: string; onClick: () => void; disabled?: boolean };
}) {
  const readcount = () => { const source = props.count; return source === undefined ? undefined : source(); };
  return (
    <View style={props.ui.styles().sectionLabel}>
      <Text style={props.ui.styles().sectionLabelText}>{props.label()}</Text>
      <Show when={readcount() !== undefined && (readcount() ?? 0) > 0}>
        <Text style={{ color: props.ui.theme().textTertiary, fontSize: 11, fontWeight: 600 }}>{readcount()}</Text>
      </Show>
      <Show when={props.action}>
        {(() => { const action = () => (props.action)!; return (
          <IconButton
            ui={props.ui}
            icon={action().icon}
            label={action().label}
            size={20}
            iconSize={12}
            disabled={action().disabled ?? false}
            onClick={action().onClick}
          />
        ); })()}
      </Show>
    </View>
  );
}

/** A tiny inline spinner substitute: three dots that the core animates through opacity. */
export function Working(props: { ui: UiContext; label: () => (string) }) {
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
      <Text style={{ fontSize: 12, color: props.ui.theme().textSecondary }}>{props.label()}</Text>
    </View>
  );
}
