/** Bounded modifier records interpreted by the Rust SwiftUI bridge. */
export interface ViewModifier {
  $type: string;
  style?: string;
  shape?: string;
  cornerRadius?: number;
  size?: string;
  color?: string;
  disabled?: boolean;
}

export type ModifierConfig = ViewModifier;
export type ButtonStyleType = "automatic" | "bordered" | "borderedProminent" | "borderless" | "glass" | "glassProminent" | "plain";
export type ButtonBorderShapeType = "automatic" | "capsule" | "roundedRectangle" | "circle";
export type ControlSizeType = "mini" | "small" | "regular" | "large" | "extraLarge";
export type LabelStyleType = "automatic" | "iconOnly" | "titleAndIcon" | "titleOnly";
export type ButtonStyleModifier = ViewModifier;
export type ButtonBorderShapeModifier = ViewModifier;
export type ControlSizeModifier = ViewModifier;
export type LabelStyleModifier = ViewModifier;
export type TintModifier = ViewModifier;
export type DisabledModifier = ViewModifier;

export function buttonStyle(style: ButtonStyleType): ViewModifier { return { $type: "buttonStyle", style }; }
export function buttonBorderShape(shape: ButtonBorderShapeType, cornerRadius?: number): ViewModifier {
  const modifier: ViewModifier = { $type: "buttonBorderShape", shape };
  if (cornerRadius !== undefined) modifier.cornerRadius = cornerRadius;
  return modifier;
}
export function controlSize(size: ControlSizeType): ViewModifier { return { $type: "controlSize", size }; }
export function labelStyle(style: LabelStyleType): ViewModifier { return { $type: "labelStyle", style }; }
export function tint(color: string): ViewModifier { return { $type: "tint", color }; }
export function disabled(value = true): ViewModifier { return { $type: "disabled", disabled: value }; }
