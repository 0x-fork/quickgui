/** A SwiftUI modifier description accepted by QuickGUI's native host. */
export interface ModifierConfig {
  $type: string;
  [key: string]: unknown;
}

export type ButtonStyleType =
  | "automatic"
  | "bordered"
  | "borderedProminent"
  | "borderless"
  | "glass"
  | "glassProminent"
  | "plain";

export type ButtonBorderShapeType = "automatic" | "capsule" | "roundedRectangle" | "circle";

export type ControlSizeType = "mini" | "small" | "regular" | "large" | "extraLarge";

export type LabelStyleType = "automatic" | "iconOnly" | "titleAndIcon" | "titleOnly";

export type ButtonStyleModifier = ModifierConfig & {
  $type: "buttonStyle";
  style: ButtonStyleType;
};

export type ButtonBorderShapeModifier = ModifierConfig & {
  $type: "buttonBorderShape";
  shape: ButtonBorderShapeType;
  cornerRadius?: number;
};

export type ControlSizeModifier = ModifierConfig & {
  $type: "controlSize";
  size: ControlSizeType;
};

export type LabelStyleModifier = ModifierConfig & {
  $type: "labelStyle";
  style: LabelStyleType;
};

export type TintModifier = ModifierConfig & {
  $type: "tint";
  color: string;
};

export type DisabledModifier = ModifierConfig & {
  $type: "disabled";
  disabled: boolean;
};

/** Button-related modifiers currently implemented by the macOS SwiftUI bridge. */
export type ViewModifier =
  | ButtonStyleModifier
  | ButtonBorderShapeModifier
  | ControlSizeModifier
  | LabelStyleModifier
  | TintModifier
  | DisabledModifier;

/** Sets the native SwiftUI button style. */
export const buttonStyle = (style: ButtonStyleType): ButtonStyleModifier => ({
  $type: "buttonStyle",
  style,
});

/** Sets the native SwiftUI button border shape. */
export const buttonBorderShape = (
  shape: ButtonBorderShapeType,
  cornerRadius?: number,
): ButtonBorderShapeModifier => ({
  $type: "buttonBorderShape",
  shape,
  ...(cornerRadius === undefined ? {} : { cornerRadius }),
});

/** Sets the native SwiftUI control size. */
export const controlSize = (size: ControlSizeType): ControlSizeModifier => ({
  $type: "controlSize",
  size,
});

/** Sets the label presentation for a button with a system image. */
export const labelStyle = (style: LabelStyleType): LabelStyleModifier => ({
  $type: "labelStyle",
  style,
});

/** Sets the SwiftUI tint color. */
export const tint = (color: string): TintModifier => ({
  $type: "tint",
  color,
});

/** Disables or enables the native SwiftUI control. */
export const disabled = (value = true): DisabledModifier => ({
  $type: "disabled",
  disabled: value,
});
