import { Button, Text, View, type JSX } from "@quickgui/solid";
import { Show, type Element } from "solid-js";
import type { Styles, Theme } from "./theme.ts";

interface UiContext {
  theme: () => Theme;
  styles: () => Styles;
}

export function PushButton(props: {
  ui: UiContext;
  kind?: "primary" | "secondary" | "danger";
  label: string;
  disabled?: boolean;
  onClick: () => void;
  style?: JSX.StyleProp;
}) {
  return (
    <Button
      aria-label={props.label}
      disabled={props.disabled ?? false}
      onClick={props.onClick}
      style={[props.ui.styles().button(props.kind ?? "secondary"), props.style]}
    >
      <Text>{props.label}</Text>
    </Button>
  );
}

export function EmptyState(props: {
  ui: UiContext;
  title: string;
  description?: string | undefined;
  children?: Element;
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
