/**
 * The property table: every style and element prop the host understands, its protocol code, and
 * the value kind that decides how it is typed, lowered at compile time, and encoded at run time.
 *
 * This file is plain data. The CLI's JSX lowering imports it to pick typed setters, and
 * `scripts/generate.ts` turns it into the `Style`/`NativeProps` interfaces plus the run-time
 * `applyStyle`/`applyProps` functions in `generated.ts`.
 */

export const KIND_LENGTH = 1;
export const KIND_NUMBER = 2;
export const KIND_STRING = 3;
/** A boolean whose `false` clears the property. */
export const KIND_BOOL = 4;
/** A boolean whose `false` is transmitted because the core defaults it to `true`. */
export const KIND_BOOL_EXPLICIT = 5;
export const KIND_COLOR = 6;
/** A solid color or one gradient declaration; `secondary` is the gradient code. */
export const KIND_BACKGROUND = 7;
export const KIND_BOX_SHADOW = 8;
/** A CSS function list (filters), joined from an array. */
export const KIND_DECLARATION = 9;
export const KIND_TRANSITION = 10;
/** One nested interaction-state style. */
export const KIND_STATE = 11;
/** A group state: one entry or a list of entries, each following a group. */
export const KIND_GROUP_STATE = 12;
export const KIND_FLEX = 13;
export const KIND_BORDER_RADIUS = 14;
/** `gridColumn`/`gridRow` shorthand; `secondary` 1 means column. */
export const KIND_GRID_PLACEMENT = 15;
export const KIND_OUTLINE = 16;
export const KIND_GRID_TEMPLATE = 17;
export const KIND_MILLISECONDS = 18;
/** A bounded component scope, value, or label. */
export const KIND_COMPONENT_VALUE = 19;
/** A JSON declaration bounded by `secondary` bytes. */
export const KIND_JSON = 20;
export const KIND_EXTENT = 21;
export const KIND_SHADER_PARAMETERS = 22;
export const KIND_TOOLTIP_TEXT = 23;
export const KIND_MENU_LINK = 24;
/** `group`: `true` opens an unnamed hover group, a string names one. */
export const KIND_HOVER_GROUP = 25;
export const KIND_TRANSFORM = 26;
export const KIND_TEXT_SHADOW = 27;
export const KIND_STRING_LIST = 28;
export const KIND_STRING_MAP = 29;
export const KIND_TERMINAL_PALETTE = 30;
/** `type="password"` on an input. */
export const KIND_INPUT_TYPE = 31;
export const KIND_KEYMAP = 32;
export const KIND_DRAG_SOURCE = 33;
export const KIND_DROP_KINDS = 34;

export interface PropertyEntry {
  /** The JSX prop / style key. */
  name: string;
  code: number;
  kind: number;
  /** Kind-specific companion: a second code, a byte limit, or a flag. */
  secondary?: number;
  /** Usable inside `style`. */
  style: boolean;
  /** TypeScript type override for the generated interfaces. */
  type?: string;
  /** Allowed string values, for editor completion. */
  values?: string[];
}

const S = (name: string, code: number, kind: number, extra: Partial<PropertyEntry> = {}): PropertyEntry => ({
  name,
  code,
  kind,
  style: true,
  ...extra,
});

const P = (name: string, code: number, kind: number, extra: Partial<PropertyEntry> = {}): PropertyEntry => ({
  name,
  code,
  kind,
  style: false,
  ...extra,
});

export const propertyTable: PropertyEntry[] = [
  S("display", 1, KIND_STRING, { values: ["none", "block", "flex", "grid"] }),
  S("flex", 0, KIND_FLEX),
  S("flexDirection", 2, KIND_STRING, { values: ["row", "row-reverse", "column", "column-reverse"] }),
  S("flexWrap", 3, KIND_STRING, { values: ["nowrap", "wrap", "wrap-reverse"] }),
  S("flexGrow", 4, KIND_NUMBER),
  S("flexShrink", 5, KIND_NUMBER),
  S("flexBasis", 6, KIND_LENGTH),
  S("alignItems", 7, KIND_STRING, { values: ["start", "flex-start", "center", "end", "flex-end", "baseline", "stretch"] }),
  S("alignSelf", 8, KIND_STRING, { values: ["start", "flex-start", "center", "end", "flex-end", "baseline", "stretch"] }),
  S("justifyContent", 9, KIND_STRING, {
    values: ["start", "flex-start", "center", "end", "flex-end", "space-between", "space-around", "space-evenly"],
  }),
  S("alignContent", 10, KIND_STRING, {
    values: ["start", "flex-start", "center", "end", "flex-end", "space-between", "space-around", "space-evenly", "normal", "stretch"],
  }),
  S("gap", 11, KIND_LENGTH),
  S("columnGap", 12, KIND_LENGTH),
  S("rowGap", 13, KIND_LENGTH),
  S("width", 14, KIND_LENGTH),
  S("height", 15, KIND_LENGTH),
  S("minWidth", 16, KIND_LENGTH),
  S("minHeight", 17, KIND_LENGTH),
  S("maxWidth", 18, KIND_LENGTH),
  S("maxHeight", 19, KIND_LENGTH),
  S("padding", 20, KIND_LENGTH),
  S("paddingTop", 21, KIND_LENGTH),
  S("paddingRight", 22, KIND_LENGTH),
  S("paddingBottom", 23, KIND_LENGTH),
  S("paddingLeft", 24, KIND_LENGTH),
  S("margin", 25, KIND_LENGTH),
  S("marginTop", 26, KIND_LENGTH),
  S("marginRight", 27, KIND_LENGTH),
  S("marginBottom", 28, KIND_LENGTH),
  S("marginLeft", 29, KIND_LENGTH),
  S("background", 30, KIND_BACKGROUND, { secondary: 259 }),
  S("backgroundColor", 30, KIND_COLOR),
  S("backgroundGradient", 30, KIND_BACKGROUND, { secondary: 259 }),
  S("color", 31, KIND_COLOR),
  S("hoverBackground", 94, KIND_BACKGROUND, { secondary: 278 }),
  S("hoverBackgroundColor", 94, KIND_COLOR),
  S("hoverColor", 95, KIND_COLOR),
  S("activeBackground", 96, KIND_BACKGROUND, { secondary: 281 }),
  S("activeBackgroundColor", 96, KIND_COLOR),
  S("activeColor", 97, KIND_COLOR),
  S("focusBackground", 284, KIND_BACKGROUND, { secondary: 286 }),
  S("focusBackgroundColor", 284, KIND_COLOR),
  S("focusColor", 285, KIND_COLOR),
  S("transition", 98, KIND_TRANSITION),
  S("opacity", 32, KIND_NUMBER),
  S("borderWidth", 33, KIND_LENGTH),
  S("borderTopWidth", 129, KIND_LENGTH),
  S("borderRightWidth", 130, KIND_LENGTH),
  S("borderBottomWidth", 131, KIND_LENGTH),
  S("borderLeftWidth", 132, KIND_LENGTH),
  S("borderColor", 34, KIND_COLOR),
  S("borderRadius", 35, KIND_BORDER_RADIUS),
  S("boxShadow", 133, KIND_BOX_SHADOW),
  S("fontSize", 36, KIND_LENGTH),
  S("fontFamily", 101, KIND_STRING),
  S("fontWeight", 37, KIND_LENGTH),
  S("lineHeight", 38, KIND_LENGTH),
  S("textAlign", 39, KIND_STRING, { values: ["left", "center", "right", "justify", "start", "end"] }),
  S("whiteSpace", 40, KIND_STRING, { values: ["normal", "nowrap"] }),
  S("textOverflow", 41, KIND_STRING, { values: ["clip", "ellipsis"] }),
  S("lineClamp", 42, KIND_NUMBER),
  S("overflow", 43, KIND_STRING, { values: ["visible", "hidden", "auto", "scroll"] }),
  S("overflowX", 44, KIND_STRING, { values: ["visible", "hidden", "auto", "scroll"] }),
  S("overflowY", 45, KIND_STRING, { values: ["visible", "hidden", "auto", "scroll"] }),
  S("cursor", 46, KIND_STRING),
  S("appRegion", 47, KIND_STRING, { values: ["drag", "no-drag"] }),
  S("position", 52, KIND_STRING, { values: ["relative", "absolute", "sticky"] }),
  S("top", 53, KIND_LENGTH),
  S("right", 54, KIND_LENGTH),
  S("bottom", 55, KIND_LENGTH),
  S("left", 56, KIND_LENGTH),
  S("userSelect", 57, KIND_STRING, { values: ["auto", "text", "none"] }),
  S("visibility", 60, KIND_STRING, { values: ["visible", "hidden"] }),
  S("aspectRatio", 61, KIND_NUMBER),
  S("gridTemplateColumns", 162, KIND_GRID_TEMPLATE),
  S("gridTemplateRows", 163, KIND_GRID_TEMPLATE),
  S("gridAutoFlow", 164, KIND_STRING, { values: ["row", "column", "row dense", "column dense"] }),
  S("gridColumn", 0, KIND_GRID_PLACEMENT, { secondary: 1 }),
  S("gridRow", 0, KIND_GRID_PLACEMENT, { secondary: 0 }),
  S("gridColumnStart", 165, KIND_NUMBER),
  S("gridColumnEnd", 166, KIND_NUMBER),
  S("gridRowStart", 168, KIND_NUMBER),
  S("gridRowEnd", 169, KIND_NUMBER),
  S("transitionProperty", 171, KIND_STRING),
  S("transitionDuration", 172, KIND_MILLISECONDS),
  S("transitionTimingFunction", 173, KIND_STRING, { values: ["linear", "ease", "ease-in", "ease-out", "ease-in-out"] }),
  S("transitionEasing", 173, KIND_STRING, { values: ["linear", "ease", "ease-in", "ease-out", "ease-in-out"] }),
  S("transitionMaxFps", 174, KIND_NUMBER),
  S("objectFit", 182, KIND_STRING, { values: ["fill", "contain", "cover", "scale-down", "none"] }),
  S("markdownCodeBackground", 68, KIND_COLOR),
  S("markdownBorderColor", 69, KIND_COLOR),
  S("markdownMutedColor", 70, KIND_COLOR),
  S("markdownLinkColor", 71, KIND_COLOR),
  S("markdownCodeTextColor", 72, KIND_COLOR),
  S("markdownBlockGap", 73, KIND_NUMBER),
  S("markdownCodeFontSize", 74, KIND_NUMBER),
  S("scrollToEndRevision", 75, KIND_NUMBER),
  S("letterSpacing", 240, KIND_LENGTH),
  S("wordSpacing", 241, KIND_LENGTH),
  S("textTransform", 242, KIND_STRING, { values: ["none", "uppercase", "lowercase", "capitalize"] }),
  S("textShadow", 243, KIND_TEXT_SHADOW),
  S("textDecoration", 244, KIND_STRING),
  S("textDecorationLine", 244, KIND_STRING),
  S("textDecorationColor", 245, KIND_COLOR),
  S("textDecorationStyle", 246, KIND_STRING, { values: ["solid", "double", "wavy"] }),
  S("textDecorationThickness", 247, KIND_LENGTH),
  S("wordBreak", 248, KIND_STRING, { values: ["normal", "break-all", "keep-all"] }),
  S("overflowWrap", 249, KIND_STRING, { values: ["normal", "anywhere", "break-word"] }),
  S("wordWrap", 249, KIND_STRING, { values: ["normal", "anywhere", "break-word"] }),
  S("hyphens", 250, KIND_STRING, { values: ["none", "manual", "auto"] }),
  S("textDirection", 251, KIND_STRING, { values: ["auto", "ltr", "rtl"] }),
  S("direction", 252, KIND_STRING, { values: ["ltr", "rtl"] }),
  S("paddingStart", 253, KIND_LENGTH),
  S("paddingInlineStart", 253, KIND_LENGTH),
  S("paddingEnd", 254, KIND_LENGTH),
  S("paddingInlineEnd", 254, KIND_LENGTH),
  S("marginStart", 255, KIND_LENGTH),
  S("marginInlineStart", 255, KIND_LENGTH),
  S("marginEnd", 256, KIND_LENGTH),
  S("marginInlineEnd", 256, KIND_LENGTH),
  S("borderStartWidth", 257, KIND_LENGTH),
  S("borderInlineStartWidth", 257, KIND_LENGTH),
  S("borderEndWidth", 258, KIND_LENGTH),
  S("borderInlineEndWidth", 258, KIND_LENGTH),
  S("borderTopLeftRadius", 260, KIND_LENGTH),
  S("borderTopRightRadius", 261, KIND_LENGTH),
  S("borderBottomRightRadius", 262, KIND_LENGTH),
  S("borderBottomLeftRadius", 263, KIND_LENGTH),
  S("borderStyle", 264, KIND_STRING, { values: ["solid", "dashed", "dotted"] }),
  S("outline", 0, KIND_OUTLINE),
  S("outlineWidth", 265, KIND_LENGTH),
  S("outlineColor", 266, KIND_COLOR),
  S("outlineOffset", 267, KIND_LENGTH),
  S("outlineStyle", 268, KIND_STRING, { values: ["solid", "dashed", "dotted", "none"] }),
  S("backgroundImage", 269, KIND_STRING),
  S("backgroundSize", 270, KIND_STRING),
  S("backgroundRepeat", 271, KIND_STRING, { values: ["no-repeat", "repeat", "repeat-x", "repeat-y"] }),
  S("backgroundPosition", 272, KIND_STRING),
  S("filter", 273, KIND_DECLARATION),
  S("backdropFilter", 274, KIND_DECLARATION),
  S("transform", 275, KIND_TRANSFORM),
  S("transformOrigin", 276, KIND_STRING),
  S("mixBlendMode", 277, KIND_STRING, {
    values: ["normal", "multiply", "screen", "darken", "lighten", "overlay", "difference", "exclusion", "hard-light", "color-dodge", "color-burn"],
  }),
  S("hoverOutline", 279, KIND_DECLARATION),
  S("hoverTransform", 280, KIND_TRANSFORM),
  S("activeOutline", 282, KIND_DECLARATION),
  S("activeTransform", 283, KIND_TRANSFORM),
  S("focusOutline", 287, KIND_DECLARATION),
  S("focusTransform", 288, KIND_TRANSFORM),
  S("scrollSnapType", 289, KIND_STRING),
  S("scrollSnapAlign", 290, KIND_STRING, { values: ["start", "center", "end"] }),
  S("scrollSnapStop", 291, KIND_STRING, { values: ["normal", "always"] }),
  S("hover", 346, KIND_STATE),
  S("active", 347, KIND_STATE),
  S("focus", 348, KIND_STATE),
  S("disabled", 349, KIND_STATE),
  S("invalid", 350, KIND_STATE),
  S("dragging", 351, KIND_STATE),
  S("dragOver", 352, KIND_STATE),
  S("groupHover", 353, KIND_GROUP_STATE),
  S("groupActive", 355, KIND_GROUP_STATE),
  S("focusWithin", 356, KIND_STATE),
  S("selected", 357, KIND_STATE),

  // Element props that are not styles.
  P("disabled", 48, KIND_BOOL_EXPLICIT),
  P("invalid", 148, KIND_BOOL_EXPLICIT),
  P("selected", 358, KIND_BOOL),
  P("group", 354, KIND_HOVER_GROUP),
  P("ariaLabel", 49, KIND_STRING),
  P("role", 50, KIND_STRING),
  P("tabIndex", 51, KIND_NUMBER),
  P("focusOnPointer", 100, KIND_BOOL_EXPLICIT),
  P("overlay", 109, KIND_BOOL),
  P("focusTrap", 110, KIND_BOOL),
  P("restorePreviousFocus", 111, KIND_BOOL),
  P("autoFocus", 112, KIND_BOOL),
  P("ariaModal", 113, KIND_BOOL),
  P("dismissOnEscape", 85, KIND_BOOL_EXPLICIT),
  P("dismissOnPointerOutside", 86, KIND_BOOL_EXPLICIT),
  P("tooltip", 153, KIND_TOOLTIP_TEXT),
  P("tooltipPlacement", 154, KIND_STRING),
  P("tooltipDelay", 155, KIND_NUMBER),
  P("tooltipGap", 156, KIND_NUMBER),
  P("tooltipViewportMargin", 157, KIND_NUMBER),
  P("hitSlop", 104, KIND_LENGTH),
  P("hitSlopTop", 105, KIND_LENGTH),
  P("hitSlopRight", 106, KIND_LENGTH),
  P("hitSlopBottom", 107, KIND_LENGTH),
  P("hitSlopLeft", 108, KIND_LENGTH),
  P("value", 62, KIND_STRING),
  P("content", 62, KIND_STRING),
  P("source", 62, KIND_STRING),
  P("src", 62, KIND_STRING),
  P("label", 62, KIND_STRING),
  P("placeholder", 63, KIND_STRING),
  P("multiline", 64, KIND_BOOL),
  P("type", 76, KIND_INPUT_TYPE, { values: ["text", "password"] }),
  P("streaming", 67, KIND_BOOL),
  P("estimatedItemHeight", 77, KIND_NUMBER),
  P("overscan", 78, KIND_NUMBER),
  P("listAlignment", 79, KIND_STRING, { values: ["top", "bottom"] }),
  P("followMode", 80, KIND_STRING, { values: ["normal", "tail"] }),
  P("anchorPlacement", 82, KIND_STRING),
  P("anchorGap", 83, KIND_NUMBER),
  P("viewportMargin", 84, KIND_NUMBER),
  P("program", 88, KIND_STRING),
  P("command", 88, KIND_STRING),
  P("arguments", 89, KIND_STRING_LIST),
  P("args", 89, KIND_STRING_LIST),
  P("workingDirectory", 90, KIND_STRING),
  P("cwd", 90, KIND_STRING),
  P("environment", 91, KIND_STRING_MAP),
  P("env", 91, KIND_STRING_MAP),
  P("scrollback", 92, KIND_NUMBER),
  P("terminalPalette", 102, KIND_TERMINAL_PALETTE),
  P("terminalCursorColor", 103, KIND_COLOR),
  P("terminalPaddingColor", 114, KIND_STRING, { values: ["background", "extend"] }),
  P("fontThicken", 115, KIND_BOOL),
  P("systemImage", 116, KIND_STRING),
  P("buttonStyle", 117, KIND_STRING),
  P("controlSize", 118, KIND_STRING),
  P("target", 121, KIND_STRING),
  P("testID", 122, KIND_STRING),
  P("modifiers", 123, KIND_JSON, { secondary: 65536, type: "string[]" }),
  P("embeddedWindow", 124, KIND_NUMBER),
  P("isPresented", 125, KIND_BOOL_EXPLICIT),
  P("attachmentAnchor", 126, KIND_STRING),
  P("arrowEdge", 127, KIND_STRING),
  P("pickerStyle", 339, KIND_STRING),
  P("datePickerComponents", 340, KIND_STRING),
  P("datePickerStyle", 341, KIND_STRING),
  P("supportsOpacity", 342, KIND_BOOL_EXPLICIT),
  P("gaugeStyle", 343, KIND_STRING),
  P("gaugeMinimumValueLabel", 344, KIND_STRING),
  P("gaugeMaximumValueLabel", 345, KIND_STRING),
  P("part", 134, KIND_STRING),
  P("scope", 137, KIND_COMPONENT_VALUE),
  P("partValue", 138, KIND_COMPONENT_VALUE),
  P("activeValue", 139, KIND_COMPONENT_VALUE),
  P("checked", 135, KIND_BOOL_EXPLICIT),
  P("indeterminate", 136, KIND_BOOL_EXPLICIT),
  P("orientation", 140, KIND_STRING, { values: ["horizontal", "vertical"] }),
  P("activateOnFocus", 141, KIND_BOOL_EXPLICIT),
  P("loopFocus", 142, KIND_BOOL_EXPLICIT),
  P("keepMounted", 143, KIND_BOOL_EXPLICIT),
  P("open", 144, KIND_BOOL_EXPLICIT),
  P("itemIndex", 145, KIND_NUMBER),
  P("headingLevel", 146, KIND_NUMBER),
  P("required", 147, KIND_BOOL_EXPLICIT),
  P("validationMessage", 149, KIND_STRING),
  P("touched", 150, KIND_BOOL_EXPLICIT),
  P("dirty", 151, KIND_BOOL_EXPLICIT),
  P("filled", 152, KIND_BOOL_EXPLICIT),
  P("variant", 158, KIND_STRING),
  P("menu", 159, KIND_STRING),
  P("min", 175, KIND_NUMBER),
  P("max", 176, KIND_NUMBER),
  P("low", 177, KIND_NUMBER),
  P("high", 178, KIND_NUMBER),
  P("optimum", 179, KIND_NUMBER),
  P("valueText", 180, KIND_STRING),
  P("pressed", 181, KIND_BOOL_EXPLICIT),
  P("shaderParameters", 183, KIND_SHADER_PARAMETERS),
  P("keymap", 197, KIND_KEYMAP),
  P("draggable", 199, KIND_DRAG_SOURCE),
  P("dropKinds", 200, KIND_DROP_KINDS),
  P("values", 203, KIND_JSON, { secondary: 65536, type: "number[]" }),
  P("step", 204, KIND_NUMBER),
  P("largeStep", 205, KIND_NUMBER),
  P("items", 206, KIND_JSON, { secondary: 65536, type: "ComponentItem[]" }),
  P("options", 208, KIND_JSON, { secondary: 524288, type: "OptionSource" }),
  P("inputValue", 209, KIND_STRING),
  P("filterMode", 210, KIND_STRING),
  P("appearance", 211, KIND_JSON, { secondary: 65536, type: "ComponentAppearance" }),
  P("columns", 212, KIND_JSON, { secondary: 2097152, type: "TableColumn[]" }),
  P("rowCount", 213, KIND_NUMBER),
  P("sortColumn", 214, KIND_STRING),
  P("sortDirection", 215, KIND_STRING, { values: ["ascending", "descending"] }),
  P("selectionMode", 216, KIND_STRING, { values: ["none", "single", "multiple"] }),
  P("selection", 217, KIND_JSON, { secondary: 2097152, type: "number[]" }),
  P("rowIndex", 218, KIND_NUMBER),
  P("columnIndex", 219, KIND_NUMBER),
  P("nodes", 220, KIND_JSON, { secondary: 2097152, type: "TreeNode[]" }),
  P("expanded", 221, KIND_JSON, { secondary: 2097152, type: "string[]" }),
  P("selectedValue", 222, KIND_COMPONENT_VALUE),
  P("setChildren", 223, KIND_JSON, { secondary: 2097152, type: "TreeNodeChildren" }),
  P("precision", 224, KIND_NUMBER),
  P("toasts", 225, KIND_JSON, { secondary: 2097152, type: "ToastDeclaration[]" }),
  P("segmentOrder", 226, KIND_STRING),
  P("segment", 227, KIND_STRING),
  P("civilValue", 228, KIND_STRING),
  P("civilMinimum", 229, KIND_STRING),
  P("civilMaximum", 230, KIND_STRING),
  P("menuCount", 231, KIND_NUMBER),
  P("firstWeekday", 232, KIND_NUMBER),
  P("rowHeight", 233, KIND_NUMBER),
  P("headerHeight", 234, KIND_NUMBER),
  P("optionGroup", 235, KIND_COMPONENT_VALUE),
  P("editing", 236, KIND_JSON, { secondary: 2097152, type: "TableEditing" }),
  P("disclosure", 237, KIND_STRING),
  P("loadingLabel", 238, KIND_STRING),
  P("delay", 292, KIND_MILLISECONDS),
  P("closeDelay", 293, KIND_MILLISECONDS),
  P("length", 294, KIND_NUMBER),
  P("mask", 295, KIND_STRING),
  P("readOnly", 296, KIND_BOOL_EXPLICIT),
  P("autoSubmit", 297, KIND_COMPONENT_VALUE),
  P("swipeDirection", 298, KIND_STRING),
  P("viewportSize", 299, KIND_EXTENT),
  P("contentSize", 300, KIND_EXTENT),
  P("overflowEdgeThreshold", 301, KIND_NUMBER),
  P("disablePointerDismissal", 302, KIND_BOOL),
  P("fit", 182, KIND_STRING, { values: ["fill", "contain", "cover", "scale-down", "none"] }),
  P("side", 303, KIND_STRING, { values: ["top", "bottom", "left", "right"] }),
  P("align", 304, KIND_STRING, { values: ["start", "center", "end"] }),
  P("sideOffset", 305, KIND_NUMBER),
  P("alignOffset", 306, KIND_NUMBER),
  P("collisionPadding", 307, KIND_NUMBER),
  P("sticky", 308, KIND_BOOL_EXPLICIT),
  P("modal", 310, KIND_BOOL_EXPLICIT),
  P("openOnHover", 311, KIND_BOOL_EXPLICIT),
  P("provider", 312, KIND_COMPONENT_VALUE),
  P("timeout", 313, KIND_MILLISECONDS),
  P("hoverable", 314, KIND_BOOL_EXPLICIT),
  P("trackCursorAxis", 315, KIND_STRING),
  P("closeOnClick", 316, KIND_BOOL_EXPLICIT),
  P("minStepsBetweenValues", 317, KIND_NUMBER),
  P("thumbAlignment", 318, KIND_STRING),
  P("format", 319, KIND_STRING),
  P("smallStep", 320, KIND_NUMBER),
  P("allowWheelScrub", 321, KIND_BOOL_EXPLICIT),
  P("snapOnStep", 322, KIND_BOOL_EXPLICIT),
  P("limit", 323, KIND_NUMBER),
  P("stackExpanded", 331, KIND_BOOL_EXPLICIT),
  P("pitch", 324, KIND_NUMBER),
  P("focusableWhenDisabled", 325, KIND_BOOL_EXPLICIT),
  P("validationMode", 326, KIND_STRING),
  P("validationDebounceTime", 327, KIND_MILLISECONDS),
  P("parent", 328, KIND_BOOL_EXPLICIT),
  P("enterDuration", 329, KIND_MILLISECONDS),
  P("exitDuration", 330, KIND_MILLISECONDS),
  P("closeParentOnEsc", 332, KIND_BOOL_EXPLICIT),
  P("href", 333, KIND_MENU_LINK),
  P("multiple", 334, KIND_BOOL_EXPLICIT),
  P("alignItemWithTrigger", 335, KIND_BOOL_EXPLICIT),
  P("autoHighlight", 336, KIND_BOOL_EXPLICIT),
  P("openOnInputClick", 337, KIND_BOOL_EXPLICIT),
  P("highlightItemOnHover", 338, KIND_BOOL_EXPLICIT),
];

/** Event handler props: the JSX name, the numeric event type, and the payload helper hint. */
export interface EventEntry {
  name: string;
  type: number;
}

export const eventTable: EventEntry[] = [
  { name: "onClick", type: 1 },
  { name: "onPress", type: 1 },
  { name: "onMouseEnter", type: 2 },
  { name: "onPointerEnter", type: 2 },
  { name: "onMouseLeave", type: 3 },
  { name: "onPointerLeave", type: 3 },
  { name: "onInput", type: 4 },
  { name: "onChange", type: 4 },
  { name: "onSubmit", type: 5 },
  { name: "onDismiss", type: 6 },
  { name: "onStatus", type: 7 },
  { name: "onTerminal", type: 7 },
  { name: "onPointer", type: 8 },
  { name: "onPresentationChange", type: 9 },
  { name: "onSelect", type: 10 },
  { name: "onMenuSelect", type: 10 },
  { name: "onKeyDown", type: 11 },
  { name: "onKeyUp", type: 12 },
  { name: "onMouseDown", type: 13 },
  { name: "onMouseUp", type: 14 },
  { name: "onMouseMove", type: 15 },
  { name: "onDoubleClick", type: 16 },
  { name: "onWheel", type: 17 },
  { name: "onContextMenu", type: 18 },
  { name: "onPinch", type: 19 },
  { name: "onRotate", type: 20 },
  { name: "onSmartMagnify", type: 21 },
  { name: "onPressure", type: 22 },
  { name: "onFocus", type: 23 },
  { name: "onBlur", type: 24 },
  { name: "onAction", type: 25 },
  { name: "onDragStart", type: 26 },
  { name: "onDragEnd", type: 27 },
  { name: "onDrop", type: 28 },
  { name: "onFilesDropped", type: 29 },
  { name: "onComponentChange", type: 30 },
  { name: "onCommit", type: 31 },
  { name: "onActivate", type: 31 },
];

/** The names of the nested interaction states `style` accepts. */
export const stateNames: string[] = [
  "hover",
  "active",
  "focus",
  "disabled",
  "invalid",
  "dragging",
  "dragOver",
  "groupHover",
  "groupActive",
  "focusWithin",
  "selected",
];

/** Element component names the JSX lowering treats as intrinsic host elements, with their tags. */
export const elementTags: { name: string; tag: number }[] = [
  { name: "View", tag: 1 },
  { name: "Text", tag: 1 },
  { name: "Button", tag: 2 },
  { name: "Input", tag: 5 },
  { name: "TextArea", tag: 5 },
  { name: "Markdown", tag: 6 },
  { name: "VirtualList", tag: 7 },
  { name: "Terminal", tag: 8 },
  { name: "Svg", tag: 9 },
  { name: "Image", tag: 16 },
  { name: "Shader", tag: 17 },
];

/** The run-time setter each value kind lowers to. */
export function setterForKind(kind: number): string {
  switch (kind) {
    case KIND_LENGTH:
      return "setLength";
    case KIND_NUMBER:
      return "setNumber";
    case KIND_STRING:
      return "setString";
    case KIND_BOOL:
      return "setBool";
    case KIND_BOOL_EXPLICIT:
      return "setExplicitBool";
    case KIND_COLOR:
      return "setColor";
    case KIND_BACKGROUND:
      return "setBackground";
    case KIND_BOX_SHADOW:
      return "setBoxShadow";
    case KIND_DECLARATION:
      return "setDeclaration";
    case KIND_TRANSITION:
      return "setTransition";
    case KIND_STATE:
      return "setStateStyle";
    case KIND_GROUP_STATE:
      return "setGroupStateStyle";
    case KIND_FLEX:
      return "setFlex";
    case KIND_BORDER_RADIUS:
      return "setBorderRadius";
    case KIND_GRID_PLACEMENT:
      return "setGridPlacement";
    case KIND_OUTLINE:
      return "setOutline";
    case KIND_GRID_TEMPLATE:
      return "setGridTemplate";
    case KIND_MILLISECONDS:
      return "setMilliseconds";
    case KIND_COMPONENT_VALUE:
      return "setComponentValue";
    case KIND_JSON:
      return "setJson";
    case KIND_EXTENT:
      return "setExtent";
    case KIND_SHADER_PARAMETERS:
      return "setShaderParameters";
    case KIND_TOOLTIP_TEXT:
      return "setTooltipText";
    case KIND_MENU_LINK:
      return "setMenuLink";
    case KIND_HOVER_GROUP:
      return "setHoverGroup";
    case KIND_TRANSFORM:
      return "setTransform";
    case KIND_TEXT_SHADOW:
      return "setTextShadow";
    case KIND_STRING_LIST:
      return "setStringList";
    case KIND_STRING_MAP:
      return "setStringMap";
    case KIND_TERMINAL_PALETTE:
      return "setTerminalPalette";
    case KIND_INPUT_TYPE:
      return "setInputType";
    case KIND_KEYMAP:
      return "setKeymap";
    case KIND_DRAG_SOURCE:
      return "setDragSource";
    case KIND_DROP_KINDS:
      return "setDropKinds";
    default:
      throw new Error("unknown QuickGUI property kind " + String(kind));
  }
}

/** The TypeScript type of a property value, for the generated interfaces. */
export function typeForEntry(entry: PropertyEntry): string {
  if (entry.type !== undefined) return entry.type;
  switch (entry.kind) {
    case KIND_LENGTH:
    case KIND_FLEX:
    case KIND_BORDER_RADIUS:
    case KIND_GRID_PLACEMENT:
    case KIND_OUTLINE:
    case KIND_MILLISECONDS:
      return "number | string";
    case KIND_NUMBER:
      return "number";
    case KIND_STRING:
      return entry.values === undefined ? "string" : entry.values.map((value) => JSON.stringify(value)).join(" | ");
    case KIND_BOOL:
    case KIND_BOOL_EXPLICIT:
      return "boolean";
    case KIND_COLOR:
      return "number | string";
    case KIND_BACKGROUND:
      return "number | string | GradientDeclaration";
    case KIND_BOX_SHADOW:
      return "string";
    case KIND_DECLARATION:
      return "string | string[]";
    case KIND_TRANSITION:
      return "number | string | TransitionDeclaration";
    case KIND_STATE:
      return "StateStyle";
    case KIND_GROUP_STATE:
      return "GroupStateStyle | GroupStateStyle[]";
    case KIND_GRID_TEMPLATE:
      return "number | string | (number | string)[]";
    case KIND_COMPONENT_VALUE:
      return "string";
    case KIND_JSON:
      return "string";
    case KIND_EXTENT:
      return "Extent";
    case KIND_SHADER_PARAMETERS:
      return "number[]";
    case KIND_TOOLTIP_TEXT:
      return "string";
    case KIND_MENU_LINK:
      return "string";
    case KIND_HOVER_GROUP:
      return "boolean | string";
    case KIND_TRANSFORM:
      return "string | string[] | TransformMatrix";
    case KIND_TEXT_SHADOW:
      return "string | TextShadowDeclaration";
    case KIND_STRING_LIST:
      return "string[]";
    case KIND_STRING_MAP:
      return "StringMap";
    case KIND_TERMINAL_PALETTE:
      return "TerminalPalette";
    case KIND_INPUT_TYPE:
      return '"text" | "password"';
    case KIND_KEYMAP:
      return "Keymap";
    case KIND_DRAG_SOURCE:
      return "boolean | DragSource";
    case KIND_DROP_KINDS:
      return "DropKind | DropKind[]";
    default:
      throw new Error("unknown QuickGUI property kind " + String(entry.kind));
  }
}
