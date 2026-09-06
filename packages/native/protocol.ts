export const PROTOCOL_VERSION = 31;
export const ROOT_NODE_ID = 0;
export const NO_ANCHOR = 0xffff_ffff;

/** Retained node kinds the host decodes. */
export const NativeNodeTag = {
  View: 1,
  Button: 2,
  Text: 3,
  Sentinel: 4,
  Input: 5,
  Markdown: 6,
  VirtualList: 7,
  Terminal: 8,
  Svg: 9,
  SwiftUIHost: 10,
  SwiftUIButton: 11,
  SwiftUIQuickGUIHost: 12,
  SwiftUIPopover: 13,
  SwiftUIPopoverTrigger: 14,
  SwiftUIPopoverContent: 15,
  Image: 16,
  Shader: 17,
  SwiftUISlider: 18,
  SwiftUIToggle: 19,
  SwiftUIProgressView: 20,
  SwiftUIStepper: 21,
  SwiftUITextField: 22,
  SwiftUIPicker: 23,
  SwiftUIDatePicker: 24,
  SwiftUIColorPicker: 25,
  SwiftUIGauge: 26,
} as const;

/** Property codes of the binary mutation protocol, shared with the Rust host. */
export const PropertyCode = {
  Display: 1,
  FlexDirection: 2,
  FlexWrap: 3,
  FlexGrow: 4,
  FlexShrink: 5,
  FlexBasis: 6,
  AlignItems: 7,
  AlignSelf: 8,
  JustifyContent: 9,
  AlignContent: 10,
  Gap: 11,
  ColumnGap: 12,
  RowGap: 13,
  Width: 14,
  Height: 15,
  MinWidth: 16,
  MinHeight: 17,
  MaxWidth: 18,
  MaxHeight: 19,
  Padding: 20,
  PaddingTop: 21,
  PaddingRight: 22,
  PaddingBottom: 23,
  PaddingLeft: 24,
  Margin: 25,
  MarginTop: 26,
  MarginRight: 27,
  MarginBottom: 28,
  MarginLeft: 29,
  BackgroundColor: 30,
  Color: 31,
  Opacity: 32,
  BorderWidth: 33,
  BorderColor: 34,
  BorderRadius: 35,
  FontSize: 36,
  FontWeight: 37,
  LineHeight: 38,
  TextAlign: 39,
  WhiteSpace: 40,
  TextOverflow: 41,
  LineClamp: 42,
  Overflow: 43,
  OverflowX: 44,
  OverflowY: 45,
  Cursor: 46,
  AppRegion: 47,
  Disabled: 48,
  AccessibilityLabel: 49,
  Role: 50,
  TabIndex: 51,
  Position: 52,
  Top: 53,
  Right: 54,
  Bottom: 55,
  Left: 56,
  UserSelect: 57,
  ClickListener: 58,
  HoverListener: 59,
  Visibility: 60,
  AspectRatio: 61,
  Value: 62,
  Placeholder: 63,
  Multiline: 64,
  InputListener: 65,
  SubmitListener: 66,
  Streaming: 67,
  MarkdownCodeBackground: 68,
  MarkdownBorderColor: 69,
  MarkdownMutedColor: 70,
  MarkdownLinkColor: 71,
  MarkdownCodeTextColor: 72,
  MarkdownBlockGap: 73,
  MarkdownCodeFontSize: 74,
  ScrollToEndRevision: 75,
  Password: 76,
  EstimatedItemHeight: 77,
  Overscan: 78,
  ListAlignment: 79,
  FollowMode: 80,
  AnchorTarget: 81,
  AnchorPlacement: 82,
  AnchorGap: 83,
  ViewportMargin: 84,
  DismissOnEscape: 85,
  DismissOnPointerOutside: 86,
  DismissListener: 87,
  TerminalProgram: 88,
  TerminalArguments: 89,
  TerminalWorkingDirectory: 90,
  TerminalEnvironment: 91,
  TerminalScrollback: 92,
  TerminalStatusListener: 93,
  HoverBackgroundColor: 94,
  HoverColor: 95,
  ActiveBackgroundColor: 96,
  ActiveColor: 97,
  Transition: 98,
  PointerListener: 99,
  FocusOnPointer: 100,
  FontFamily: 101,
  TerminalPalette: 102,
  TerminalCursorColor: 103,
  HitSlop: 104,
  HitSlopTop: 105,
  HitSlopRight: 106,
  HitSlopBottom: 107,
  HitSlopLeft: 108,
  Overlay: 109,
  FocusTrap: 110,
  RestorePreviousFocus: 111,
  AutoFocus: 112,
  AccessibilityModal: 113,
  TerminalPaddingColor: 114,
  TerminalFontThicken: 115,
  SwiftUISystemImage: 116,
  SwiftUIButtonStyle: 117,
  SwiftUIControlSize: 118,
  SwiftUIMatchContentsHorizontal: 119,
  SwiftUIMatchContentsVertical: 120,
  SwiftUITarget: 121,
  SwiftUITestId: 122,
  SwiftUIModifiers: 123,
  SwiftUIEmbeddedWindow: 124,
  SwiftUIIsPresented: 125,
  SwiftUIAttachmentAnchor: 126,
  SwiftUIArrowEdge: 127,
  SwiftUIPresentationListener: 128,
  BorderTopWidth: 129,
  BorderRightWidth: 130,
  BorderBottomWidth: 131,
  BorderLeftWidth: 132,
  BoxShadow: 133,
  Part: 134,
  Checked: 135,
  Indeterminate: 136,
  Scope: 137,
  PartValue: 138,
  ActiveValue: 139,
  Orientation: 140,
  ActivateOnFocus: 141,
  LoopFocus: 142,
  KeepMounted: 143,
  Open: 144,
  ItemIndex: 145,
  HeadingLevel: 146,
  Required: 147,
  Invalid: 148,
  ValidationMessage: 149,
  Touched: 150,
  Dirty: 151,
  Filled: 152,
  Tooltip: 153,
  TooltipPlacement: 154,
  TooltipDelay: 155,
  TooltipGap: 156,
  TooltipViewportMargin: 157,
  Variant: 158,
  Menu: 159,
  SelectListener: 160,
  Controls: 161,
  GridTemplateColumns: 162,
  GridTemplateRows: 163,
  GridAutoFlow: 164,
  GridColumnStart: 165,
  GridColumnEnd: 166,
  GridColumnSpan: 167,
  GridRowStart: 168,
  GridRowEnd: 169,
  GridRowSpan: 170,
  TransitionProperties: 171,
  TransitionDuration: 172,
  TransitionEasing: 173,
  TransitionMaxFps: 174,
  Minimum: 175,
  Maximum: 176,
  Low: 177,
  High: 178,
  Optimum: 179,
  ValueText: 180,
  Pressed: 181,
  ObjectFit: 182,
  ShaderParameters: 183,
  KeyDownListener: 184,
  KeyUpListener: 185,
  MouseDownListener: 186,
  MouseUpListener: 187,
  MouseMoveListener: 188,
  DoubleClickListener: 189,
  ScrollListener: 190,
  ContextMenuListener: 191,
  PinchListener: 192,
  RotationListener: 193,
  SmartMagnifyListener: 194,
  PressureListener: 195,
  FocusListener: 196,
  Keymap: 197,
  ActionListener: 198,
  Draggable: 199,
  DropKinds: 200,
  DragListener: 201,
  DropListener: 202,
  Values: 203,
  Step: 204,
  LargeStep: 205,
  Items: 206,
  ComponentChangeListener: 207,
  Options: 208,
  InputValue: 209,
  FilterMode: 210,
  Appearance: 211,
  Columns: 212,
  RowCount: 213,
  SortColumn: 214,
  SortDirection: 215,
  SelectionMode: 216,
  Selection: 217,
  RowIndex: 218,
  ColumnIndex: 219,
  Nodes: 220,
  Expanded: 221,
  SelectedValue: 222,
  SetChildren: 223,
  Precision: 224,
  Toasts: 225,
  SegmentOrder: 226,
  Segment: 227,
  CivilValue: 228,
  CivilMinimum: 229,
  CivilMaximum: 230,
  MenuCount: 231,
  FirstWeekday: 232,
  RowHeight: 233,
  HeaderHeight: 234,
  Group: 235,
  Editing: 236,
  Disclosure: 237,
  LoadingLabel: 238,
  CommitListener: 239,
  LetterSpacing: 240,
  WordSpacing: 241,
  TextTransform: 242,
  TextShadow: 243,
  TextDecorationLine: 244,
  TextDecorationColor: 245,
  TextDecorationStyle: 246,
  TextDecorationThickness: 247,
  WordBreak: 248,
  OverflowWrap: 249,
  Hyphens: 250,
  TextDirection: 251,
  Direction: 252,
  PaddingStart: 253,
  PaddingEnd: 254,
  MarginStart: 255,
  MarginEnd: 256,
  BorderStartWidth: 257,
  BorderEndWidth: 258,
  BackgroundGradient: 259,
  BorderTopLeftRadius: 260,
  BorderTopRightRadius: 261,
  BorderBottomRightRadius: 262,
  BorderBottomLeftRadius: 263,
  BorderStyle: 264,
  OutlineWidth: 265,
  OutlineColor: 266,
  OutlineOffset: 267,
  OutlineStyle: 268,
  BackgroundImage: 269,
  BackgroundSize: 270,
  BackgroundRepeat: 271,
  BackgroundPosition: 272,
  Filter: 273,
  BackdropFilter: 274,
  Transform: 275,
  TransformOrigin: 276,
  MixBlendMode: 277,
  HoverBackgroundGradient: 278,
  HoverOutline: 279,
  HoverTransform: 280,
  ActiveBackgroundGradient: 281,
  ActiveOutline: 282,
  ActiveTransform: 283,
  FocusBackgroundColor: 284,
  FocusColor: 285,
  FocusBackgroundGradient: 286,
  FocusOutline: 287,
  FocusTransform: 288,
  ScrollSnapType: 289,
  ScrollSnapAlign: 290,
  ScrollSnapStop: 291,
  Delay: 292,
  CloseDelay: 293,
  Length: 294,
  Mask: 295,
  ReadOnly: 296,
  AutoSubmit: 297,
  SwipeDirection: 298,
  ViewportSize: 299,
  ContentSize: 300,
  OverflowEdgeThreshold: 301,
  DisablePointerDismissal: 302,
  Side: 303,
  Align: 304,
  SideOffset: 305,
  AlignOffset: 306,
  CollisionPadding: 307,
  Sticky: 308,
  AnchorPoint: 309,
  Modal: 310,
  OpenOnHover: 311,
  Provider: 312,
  Timeout: 313,
  Hoverable: 314,
  TrackCursorAxis: 315,
  CloseOnClick: 316,
  MinStepsBetweenValues: 317,
  ThumbAlignment: 318,
  Format: 319,
  SmallStep: 320,
  AllowWheelScrub: 321,
  SnapOnStep: 322,
  Limit: 323,
  Pitch: 324,
  FocusableWhenDisabled: 325,
  ValidationMode: 326,
  ValidationDebounceTime: 327,
  Parent: 328,
  EnterDuration: 329,
  ExitDuration: 330,
  StackExpanded: 331,
  CloseParentOnEsc: 332,
  Href: 333,
  Multiple: 334,
  AlignItemWithTrigger: 335,
  AutoHighlight: 336,
  OpenOnInputClick: 337,
  HighlightItemOnHover: 338,
  SwiftUIPickerStyle: 339,
  SwiftUIDatePickerComponents: 340,
  SwiftUIDatePickerStyle: 341,
  SwiftUIColorSupportsOpacity: 342,
  SwiftUIGaugeStyle: 343,
  SwiftUIGaugeMinimumValueLabel: 344,
  SwiftUIGaugeMaximumValueLabel: 345,
  HoverStyle: 346,
  ActiveStyle: 347,
  FocusStyle: 348,
  DisabledStyle: 349,
  InvalidStyle: 350,
  DraggingStyle: 351,
  DragOverStyle: 352,
  GroupHoverStyle: 353,
  HoverGroup: 354,
  GroupActiveStyle: 355,
  FocusWithinStyle: 356,
  SelectedStyle: 357,
  Selected: 358,
} as const;

/**
 * Compound part names adopted from the Rust core's unstyled part descriptors.
 *
 * The renderer declares the part ahead of time on an ordinary `view`/`button` node. The Rust
 * binding rebuilds the matching core descriptor and applies its exact identity, semantics,
 * keyboard behavior, and mount policy; the JavaScript layer never reimplements them.
 */
export const NativePart = {
  Checkbox: "checkbox",
  CheckboxIndicator: "checkbox-indicator",
  Radio: "radio",
  RadioIndicator: "radio-indicator",
  RadioGroup: "radio-group",
  Switch: "switch",
  SwitchThumb: "switch-thumb",
  Tabs: "tabs",
  TabsList: "tabs-list",
  Tab: "tab",
  TabIndicator: "tab-indicator",
  TabPanel: "tab-panel",
  Collapsible: "collapsible",
  CollapsibleTrigger: "collapsible-trigger",
  CollapsiblePanel: "collapsible-panel",
  Accordion: "accordion",
  AccordionItem: "accordion-item",
  AccordionHeader: "accordion-header",
  AccordionTrigger: "accordion-trigger",
  AccordionPanel: "accordion-panel",
  Field: "field",
  FieldLabel: "field-label",
  FieldPassiveLabel: "field-passive-label",
  FieldControl: "field-control",
  FieldDescription: "field-description",
  FieldError: "field-error",
  Fieldset: "fieldset",
  FieldsetLegend: "fieldset-legend",
  FieldsetDescription: "fieldset-description",
  FieldsetControl: "fieldset-control",
  DialogTrigger: "dialog-trigger",
  Dialog: "dialog",
  DialogBackdrop: "dialog-backdrop",
  DialogPopup: "dialog-popup",
  DialogTitle: "dialog-title",
  DialogDescription: "dialog-description",
  DialogClose: "dialog-close",
  PopoverMenuTrigger: "popover-menu-trigger",
  PopoverMenuPopup: "popover-menu-popup",
  ContextMenuTrigger: "context-menu-trigger",
  Progress: "progress",
  ProgressIndicator: "progress-indicator",
  Meter: "meter",
  MeterIndicator: "meter-indicator",
  Toggle: "toggle",
  ToggleIndicator: "toggle-indicator",
  Slider: "slider",
  SliderTrack: "slider-track",
  SliderRange: "slider-range",
  SliderThumb: "slider-thumb",
  Splitter: "splitter",
  SplitterPane: "splitter-pane",
  SplitterHandle: "splitter-handle",
  Toolbar: "toolbar",
  ToolbarItem: "toolbar-item",
  ToggleGroup: "toggle-group",
  ToggleGroupItem: "toggle-group-item",
  Select: "select",
  Combobox: "combobox",
  Autocomplete: "autocomplete",
  Option: "option",
  Table: "table",
  TableHeader: "table-header",
  TableRow: "table-row",
  TableCell: "table-cell",
  Tree: "tree",
  TreeRow: "tree-row",
  NumberField: "number-field",
  NumberFieldInput: "number-field-input",
  NumberFieldIncrement: "number-field-increment",
  NumberFieldDecrement: "number-field-decrement",
  DateField: "date-field",
  DateFieldSegment: "date-field-segment",
  TimeField: "time-field",
  TimeFieldSegment: "time-field-segment",
  Calendar: "calendar",
  CalendarWeek: "calendar-week",
  CalendarDay: "calendar-day",
  Menubar: "menubar",
  MenubarItem: "menubar-item",
  ToastViewport: "toast-viewport",
  Toast: "toast",
  ToastTitle: "toast-title",
  ToastDescription: "toast-description",
  ToastAction: "toast-action",
  ToastClose: "toast-close",
  Separator: "separator",
  Avatar: "avatar",
  AvatarImage: "avatar-image",
  AvatarFallback: "avatar-fallback",
  CheckboxGroup: "checkbox-group",
  CheckboxGroupItem: "checkbox-group-item",
  CheckboxGroupIndicator: "checkbox-group-indicator",
  CheckboxGroupParent: "checkbox-group-parent",
  PreviewCard: "preview-card",
  PreviewCardTrigger: "preview-card-trigger",
  PreviewCardPortal: "preview-card-portal",
  PreviewCardPositioner: "preview-card-positioner",
  PreviewCardPopup: "preview-card-popup",
  PreviewCardArrow: "preview-card-arrow",
  PreviewCardBackdrop: "preview-card-backdrop",
  ScrollArea: "scroll-area",
  ScrollAreaViewport: "scroll-area-viewport",
  ScrollAreaContent: "scroll-area-content",
  ScrollAreaScrollbar: "scroll-area-scrollbar",
  ScrollAreaThumb: "scroll-area-thumb",
  ScrollAreaCorner: "scroll-area-corner",
  OtpField: "otp-field",
  OtpFieldInput: "otp-field-input",
  OtpFieldSeparator: "otp-field-separator",
  Drawer: "drawer",
  DrawerTrigger: "drawer-trigger",
  DrawerPortal: "drawer-portal",
  DrawerBackdrop: "drawer-backdrop",
  DrawerViewport: "drawer-viewport",
  DrawerPopup: "drawer-popup",
  DrawerContent: "drawer-content",
  DrawerTitle: "drawer-title",
  DrawerDescription: "drawer-description",
  DrawerClose: "drawer-close",
  DrawerSwipeArea: "drawer-swipe-area",
  NavigationMenu: "navigation-menu",
  NavigationMenuList: "navigation-menu-list",
  NavigationMenuItem: "navigation-menu-item",
  NavigationMenuTrigger: "navigation-menu-trigger",
  NavigationMenuIcon: "navigation-menu-icon",
  NavigationMenuPortal: "navigation-menu-portal",
  NavigationMenuPositioner: "navigation-menu-positioner",
  NavigationMenuPopup: "navigation-menu-popup",
  NavigationMenuViewport: "navigation-menu-viewport",
  NavigationMenuContent: "navigation-menu-content",
  NavigationMenuArrow: "navigation-menu-arrow",
  NavigationMenuBackdrop: "navigation-menu-backdrop",
  NavigationMenuLink: "navigation-menu-link",
  Popover: "popover",
  PopoverTrigger: "popover-trigger",
  PopoverPortal: "popover-portal",
  PopoverPositioner: "popover-positioner",
  PopoverPopup: "popover-popup",
  PopoverArrow: "popover-arrow",
  PopoverViewport: "popover-viewport",
  PopoverBackdrop: "popover-backdrop",
  PopoverTitle: "popover-title",
  PopoverDescription: "popover-description",
  PopoverClose: "popover-close",
  TooltipProvider: "tooltip-provider",
  Tooltip: "tooltip",
  TooltipTrigger: "tooltip-trigger",
  TooltipPortal: "tooltip-portal",
  TooltipPositioner: "tooltip-positioner",
  TooltipPopup: "tooltip-popup",
  TooltipArrow: "tooltip-arrow",
  SliderLabel: "slider-label",
  SliderValue: "slider-value",
  SliderControl: "slider-control",
  SliderIndicator: "slider-indicator",
  NumberFieldGroup: "number-field-group",
  NumberFieldScrubArea: "number-field-scrub-area",
  NumberFieldScrubAreaCursor: "number-field-scrub-area-cursor",
  ProgressTrack: "progress-track",
  ProgressLabel: "progress-label",
  ProgressValue: "progress-value",
  MeterTrack: "meter-track",
  MeterLabel: "meter-label",
  MeterValue: "meter-value",
  ToastPortal: "toast-portal",
  ToastPositioner: "toast-positioner",
  ToastContent: "toast-content",
  ToolbarButton: "toolbar-button",
  ToolbarLink: "toolbar-link",
  ToolbarInput: "toolbar-input",
  ToolbarGroup: "toolbar-group",
  ToolbarSeparator: "toolbar-separator",
  FieldItem: "field-item",
  FieldValidity: "field-validity",
  DialogViewport: "dialog-viewport",
  Menu: "menu",
  MenuTrigger: "menu-trigger",
  MenuPortal: "menu-portal",
  MenuBackdrop: "menu-backdrop",
  MenuPositioner: "menu-positioner",
  MenuPopup: "menu-popup",
  MenuArrow: "menu-arrow",
  MenuItem: "menu-item",
  MenuLinkItem: "menu-link-item",
  MenuSubmenuRoot: "menu-submenu-root",
  MenuSubmenuTrigger: "menu-submenu-trigger",
  MenuGroup: "menu-group",
  MenuGroupLabel: "menu-group-label",
  MenuRadioGroup: "menu-radio-group",
  MenuRadioItem: "menu-radio-item",
  MenuRadioItemIndicator: "menu-radio-item-indicator",
  MenuCheckboxItem: "menu-checkbox-item",
  MenuCheckboxItemIndicator: "menu-checkbox-item-indicator",
  MenuSeparator: "menu-separator",
  SelectLabel: "select-label",
  SelectValue: "select-value",
  SelectIcon: "select-icon",
  SelectBackdrop: "select-backdrop",
  SelectPortal: "select-portal",
  SelectPositioner: "select-positioner",
  SelectPopup: "select-popup",
  SelectArrow: "select-arrow",
  SelectList: "select-list",
  SelectItem: "select-item",
  SelectItemText: "select-item-text",
  SelectItemIndicator: "select-item-indicator",
  SelectGroup: "select-group",
  SelectGroupLabel: "select-group-label",
  SelectSeparator: "select-separator",
  SelectScrollUpArrow: "select-scroll-up-arrow",
  SelectScrollDownArrow: "select-scroll-down-arrow",
  ComboboxLabel: "combobox-label",
  ComboboxValue: "combobox-value",
  ComboboxIcon: "combobox-icon",
  ComboboxInputGroup: "combobox-input-group",
  ComboboxClear: "combobox-clear",
  ComboboxTrigger: "combobox-trigger",
  ComboboxChips: "combobox-chips",
  ComboboxChip: "combobox-chip",
  ComboboxChipRemove: "combobox-chip-remove",
  ComboboxBackdrop: "combobox-backdrop",
  ComboboxPortal: "combobox-portal",
  ComboboxPositioner: "combobox-positioner",
  ComboboxPopup: "combobox-popup",
  ComboboxArrow: "combobox-arrow",
  ComboboxStatus: "combobox-status",
  ComboboxEmpty: "combobox-empty",
  ComboboxList: "combobox-list",
  ComboboxRow: "combobox-row",
  ComboboxItem: "combobox-item",
  ComboboxItemIndicator: "combobox-item-indicator",
  ComboboxGroup: "combobox-group",
  ComboboxGroupLabel: "combobox-group-label",
  ComboboxCollection: "combobox-collection",
  ComboboxSeparator: "combobox-separator",
} as const;

export type NativePartName = (typeof NativePart)[keyof typeof NativePart];

/** Longest compound scope key or item value accepted by the Rust binding. */
export const MAX_COMPONENT_VALUE_BYTES = 256;

/** Longest tooltip label retained by the Rust binding. */
export const MAX_TOOLTIP_TEXT_BYTES = 1024;

/** Longest bounded menu declaration accepted by the Rust binding. */
export const MAX_MENU_JSON_BYTES = 512 * 1024;

/** Longest bounded accelerator keymap accepted by the Rust binding. */
export const MAX_KEYMAP_JSON_BYTES = 64 * 1024;

/** Longest bounded `values` or `items` component declaration accepted by the Rust binding. */
export const MAX_COMPONENT_JSON_BYTES = 64 * 1024;

/** Most numbers the Rust binding decodes from one `values` declaration. */
export const MAX_COMPONENT_VALUES = 64;

/** Most entries the Rust binding decodes from one `items` declaration. */
export const MAX_COMPONENT_ITEMS = 256;

/** Longest bounded drag declaration accepted by the Rust binding. */
export const MAX_DRAG_JSON_BYTES = 64 * 1024;

/** Longest bounded option source accepted by the Rust binding. */
export const MAX_OPTIONS_JSON_BYTES = 512 * 1024;

/** Most options the Rust binding decodes from one declared source. */
export const MAX_DECLARED_OPTIONS = 4096;

/** Longest bounded column, node, or selection declaration accepted by the Rust binding. */
export const MAX_COLLECTION_JSON_BYTES = 2 * 1024 * 1024;

/** Most tree nodes the Rust binding decodes from one declared source. */
export const MAX_DECLARED_TREE_NODES = 65_536;

/** Most columns the Rust core retains for one table. */
export const MAX_TABLE_COLUMNS = 512;

/** Most rows one declared table may address. */
export const MAX_TABLE_ROWS = 1_000_000;

/** Most toasts the Rust core keeps queued in one viewport. */
export const MAX_TOASTS = 8;

/** Most menus one declared in-window menubar retains. */
export const MAX_MENUBAR_MENUS = 64;

/**
 * Longest gradient, filter, transform, outline, or text-shadow declaration the Rust binding parses.
 *
 * Each of these is a fixed-size core value, so a longer declaration can only be malformed; the
 * renderer rejects it before it reaches the boundary.
 */
export const MAX_STYLE_DECLARATION_BYTES = 4096;

/** Longest nested interaction-state style declaration (`hover`, `dragOver`, …) the Rust binding parses. */
export const MAX_STATE_STYLE_JSON_BYTES = 16 * 1024;

/** Longest group name a `group` prop declares or a `groupHover`/`groupActive` follows. */
export const MAX_HOVER_GROUP_NAME_BYTES = 256;

/** Most `groupHover` and `groupActive` entries one element follows, counted together. */
export const MAX_GROUP_STYLES_PER_ELEMENT = 8;

/** Most color stops the Rust core retains for one gradient. */
export const MAX_GRADIENT_STOPS = 8;

/** Most filters the Rust core retains in one element's chain. */
export const MAX_FILTERS_PER_ELEMENT = 8;

/** Longest avatar fallback deadline, in milliseconds. */
export const MAX_AVATAR_FALLBACK_DELAY_MS = 10_000;

/** Declared and checked values retained by one checkbox group. */
export const MAX_CHECKBOX_GROUP_VALUES = 256;

/** Longest preview-card open or close deadline, in milliseconds. */
export const MAX_PREVIEW_CARD_DELAY_MS = 10_000;

/** Largest scroll-area overflow edge threshold, in logical pixels. */
export const MAX_SCROLL_AREA_OVERFLOW_THRESHOLD = 256;

/** Most slots one OTP field retains. */
export const MAX_OTP_LENGTH = 12;

/** Most snap points one drawer retains. */
export const MAX_DRAWER_SNAP_POINTS = 8;

/** Most top-level items in one navigation menu. */
export const MAX_NAVIGATION_MENU_ITEMS = 64;

/** Longest navigation-menu open or close deadline, in milliseconds. */
export const MAX_NAVIGATION_MENU_DELAY_MS = 10_000;

/** Largest popover or tooltip side offset, in logical pixels. */
export const MAX_ANCHOR_SIDE_OFFSET = 256;

/** Largest popover cross-axis align offset, in logical pixels. */
export const MAX_ANCHOR_ALIGN_OFFSET = 4096;

/** Largest popover or tooltip collision padding, in logical pixels. */
export const MAX_ANCHOR_COLLISION_PADDING = 512;

/** Longest popover hover open or close deadline, in milliseconds. */
export const MAX_POPOVER_HOVER_DELAY_MS = 10_000;

/** Longest tooltip open or close deadline, in milliseconds. */
export const MAX_TOOLTIP_DELAY_MS = 10_000;

/** Longest tooltip provider warm-group timeout, in milliseconds. */
export const MAX_TOOLTIP_GROUP_TIMEOUT_MS = 10_000;

/** Longest toast auto-dismiss duration, in milliseconds. */
export const MAX_TOAST_DURATION_MS = 60_000;

/** Largest toast swipe-dismissal threshold, in logical pixels. */
export const MAX_TOAST_SWIPE_THRESHOLD = 512;

/** Longest field validation debounce, in milliseconds. */
export const MAX_FIELD_VALIDATION_DEBOUNCE_MS = 10_000;

/** Longest dialog enter or exit transition, in milliseconds. */
export const MAX_DIALOG_TRANSITION_MS = 10_000;

/** Largest number-field scrub sensitivity, in logical pixels per step. */
export const MAX_NUMBER_FIELD_SCRUB_SENSITIVITY = 256;

/** Longest menu hover open or close deadline, in milliseconds. */
export const MAX_MENU_HOVER_DELAY_MS = 10_000;

/** Longest bounded `Menu.LinkItem` destination accepted by the Rust binding. */
export const MAX_MENU_LINK_BYTES = 8 * 1024;

/** Most rows one declared in-window menu level retains. */
export const MAX_MENU_ITEMS = 2_048;

/** Most values one multiple select retains. */
export const MAX_SELECT_VALUES = 256;

/** Most chips one multiple combobox retains. */
export const MAX_COMBOBOX_VALUES = 64;

export type NativeNodeTagValue = number;
export type PropertyCodeValue = number;
export type NativePropertyValue = boolean | number | string | null;

export class MutationBatch {
  _bytes = new Uint8Array(256);
  _view = new DataView(this._bytes.buffer);
  _length = 10;
  _count = 0;

  get empty(): boolean {
    return this._count === 0;
  }

  get mutationCount(): number {
    return this._count;
  }

  createElement(id: number, tag: number): void {
    this._op(1);
    this._u32(id);
    this._u8(tag);
  }

  createText(id: number, value: string): void {
    this._op(2);
    this._u32(id);
    this._string(value);
  }

  createSentinel(id: number): void {
    this._op(3);
    this._u32(id);
  }

  clearProperty(id: number, property: number): void {
    this._op(4);
    this._u32(id);
    this._u16(property);
    this._u8(0);
  }

  setBoolean(id: number, property: number, value: boolean): void {
    this._op(4);
    this._u32(id);
    this._u16(property);
    this._u8(1);
    this._u8(value ? 1 : 0);
  }

  setNumber(id: number, property: number, value: number): void {
    if (!Number.isFinite(value)) {
      throw new TypeError("QuickGUI property " + String(property) + " must be finite");
    }
    this._op(4);
    this._u32(id);
    this._u16(property);
    this._u8(2);
    this._f32(value);
  }

  setColor(id: number, property: number, value: number): void {
    this._op(4);
    this._u32(id);
    this._u16(property);
    this._u8(3);
    this._u32(value >>> 0);
  }

  setString(id: number, property: number, value: string): void {
    this._op(4);
    this._u32(id);
    this._u16(property);
    this._u8(4);
    this._string(value);
  }

  /** Encode one typed value; `null` withdraws the property. */
  setProperty(id: number, property: number, value: NativePropertyValue, color: boolean): void {
    if (value === null) {
      this.clearProperty(id, property);
    } else if (typeof value === "boolean") {
      this.setBoolean(id, property, value);
    } else if (typeof value === "number") {
      if (color) this.setColor(id, property, value);
      else this.setNumber(id, property, value);
    } else {
      this.setString(id, property, value);
    }
  }

  replaceText(id: number, value: string): void {
    this._op(5);
    this._u32(id);
    this._string(value);
  }

  insert(parent: number, child: number, before: number): void {
    this._op(6);
    this._u32(parent);
    this._u32(child);
    this._u32(before);
  }

  remove(parent: number, child: number): void {
    this._op(7);
    this._u32(parent);
    this._u32(child);
  }

  cleanup(parent: number, children: readonly number[]): void {
    this._op(8);
    this._u32(parent);
    this._u32(children.length);
    for (const child of children) this._u32(child);
  }

  /** Append every mutation of a detached subtree's buffer, preserving its order. */
  append(other: MutationBatch): void {
    const bytes = other.body();
    this._ensure(bytes.length);
    this._bytes.set(bytes, this._length);
    this._length += bytes.length;
    this._count += other._count;
  }

  /** The encoded mutations without the header. */
  body(): Uint8Array {
    return this._bytes.subarray(10, this._length);
  }

  /** The complete encoded batch, or an empty span when nothing was recorded. */
  finish(): Uint8Array {
    if (this._count === 0) return new Uint8Array(0);
    this._bytes[0] = 0x51;
    this._bytes[1] = 0x47;
    this._bytes[2] = 0x4d;
    this._bytes[3] = 0x42;
    this._view.setUint16(4, PROTOCOL_VERSION, true);
    this._view.setUint32(6, this._count, true);
    return this._bytes.subarray(0, this._length);
  }

  _op(opcode: number): void {
    this._count++;
    this._u8(opcode);
  }

  _ensure(additional: number): void {
    const required = this._length + additional;
    if (required <= this._bytes.length) return;
    let capacity = this._bytes.length;
    while (capacity < required) capacity *= 2;
    const next = new Uint8Array(capacity);
    next.set(this._bytes.subarray(0, this._length));
    this._bytes = next;
    this._view = new DataView(next.buffer);
  }

  _u8(value: number): void {
    this._ensure(1);
    this._view.setUint8(this._length, value);
    this._length += 1;
  }

  _u16(value: number): void {
    this._ensure(2);
    this._view.setUint16(this._length, value, true);
    this._length += 2;
  }

  _u32(value: number): void {
    if (!Number.isInteger(value) || value < 0 || value > 0xffff_ffff) {
      throw new RangeError("QuickGUI protocol value " + String(value) + " is not a u32");
    }
    this._ensure(4);
    this._view.setUint32(this._length, value, true);
    this._length += 4;
  }

  _f32(value: number): void {
    this._ensure(4);
    this._view.setFloat32(this._length, value, true);
    this._length += 4;
  }

  _string(value: string): void {
    const bytes = new TextEncoder().encode(value);
    this._u32(bytes.length);
    this._ensure(bytes.length);
    this._bytes.set(bytes, this._length);
    this._length += bytes.length;
  }
}
