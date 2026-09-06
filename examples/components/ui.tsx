/** Small presentational helpers shared by every demo. */

import type { NativeNode } from "@quickgui/native";
import { Button, Show, Text, View } from "@quickgui/ui";
import type { Dynamic } from "@quickgui/ui/parts";

import { controlStyle, p } from "./theme.ts";

export interface PanelProps {
  title: string;
  hint?: string;
  children?: () => NativeNode;
}

export function Panel(props: PanelProps): NativeNode {
  const children = props.children;
  return (
    <View
      style={{
        display: "flex",
        flexDirection: "column",
        gap: 12,
        padding: 16,
        backgroundColor: p().panel,
        borderColor: p().border,
        borderWidth: 1,
        borderRadius: 12,
      }}
    >
      <Text style={{ color: p().faint, fontSize: 11, letterSpacing: 0.8, textTransform: "uppercase", fontWeight: 700 }}>
        {props.title}
      </Text>
      <Show when={() => props.hint !== undefined}>
        <Text style={{ fontSize: 12, color: p().muted, lineHeight: 17 }}>{props.hint ?? ""}</Text>
      </Show>
      {children === undefined ? undefined : children()}
    </View>
  );
}

export interface ChildrenProps {
  children?: () => NativeNode;
}

export function Row(props: ChildrenProps): NativeNode {
  const children = props.children;
  return (
    <View style={{ display: "flex", flexDirection: "row", alignItems: "center", gap: 10, flexWrap: "wrap" }}>
      {children === undefined ? undefined : children()}
    </View>
  );
}

export interface ColProps {
  gap?: number;
  children?: () => NativeNode;
}

export function Col(props: ColProps): NativeNode {
  const children = props.children;
  return (
    <View style={{ display: "flex", flexDirection: "column", gap: props.gap ?? 8 }}>
      {children === undefined ? undefined : children()}
    </View>
  );
}

export interface TextProps {
  /** A fixed string, or an accessor the text follows reactively. */
  text: Dynamic<string>;
}

function readText(text: Dynamic<string>): string {
  return typeof text === "function" ? text() : text;
}

/** The state the core reported, printed as text. Every demo ends with one of these. */
export function Note(props: TextProps): NativeNode {
  return (
    <Text style={{ fontSize: 12, color: p().muted, fontFamily: "ui-monospace, Menlo, monospace", lineHeight: 18 }}>
      {readText(props.text)}
    </Text>
  );
}

export function Label(props: TextProps): NativeNode {
  return <Text style={{ fontSize: 12, color: p().ink }}>{readText(props.text)}</Text>;
}

export function Muted(props: TextProps): NativeNode {
  return <Text style={{ fontSize: 12, color: p().muted }}>{readText(props.text)}</Text>;
}

export interface BtnProps {
  label: Dynamic<string>;
  onClick: () => void;
  primary?: boolean;
  disabled?: boolean;
}

export function Btn(props: BtnProps): NativeNode {
  const primary = props.primary === true;
  return (
    <Button
      disabled={props.disabled === true}
      style={[
        controlStyle(),
        {
          backgroundColor: primary ? p().accent : p().control,
          borderColor: primary ? p().accent : p().border,
          // A filled button darkens its own fill on hover instead of taking the neutral hover
          // background, which would put its light label on a light surface.
          hover: { backgroundColor: primary ? p().accentHover : p().controlHover },
          opacity: props.disabled === true ? 0.5 : 1,
        },
      ]}
      onClick={() => props.onClick()}
    >
      <Text style={{ fontSize: 12, color: primary ? p().onAccent : p().ink }}>{readText(props.label)}</Text>
    </Button>
  );
}
