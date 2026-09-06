import type { DocsOutlineItem } from './docs'

export type ComponentDocKind = 'ui' | 'swift-ui'

export type ComponentDocSection =
  | 'Primitives'
  | 'Forms & Controls'
  | 'Layout & Data'
  | 'Overlays'
  | 'Menus & Navigation'
  | 'SwiftUI Components'

export interface ComponentDoc {
  kind: ComponentDocKind
  slug: string
  name: string
  section: ComponentDocSection
  description: string
  parts: readonly string[]
  keyProps: readonly string[]
}

export interface ComponentNavGroup {
  title: ComponentDocSection
  items: readonly ComponentDoc[]
}

export const ALL_COMPONENT_DOCS = [
  {
    "kind": "ui",
    "slug": "view",
    "name": "View",
    "section": "Primitives",
    "description": "A general-purpose retained container for layout, paint, pointer input, and accessibility.",
    "parts": [],
    "keyProps": [
      "style",
      "children",
      "onClick",
      "ref"
    ]
  },
  {
    "kind": "ui",
    "slug": "text",
    "name": "Text",
    "section": "Primitives",
    "description": "Shapes and paints Unicode text with inherited typography, selection, and accessibility.",
    "parts": [],
    "keyProps": [
      "style",
      "children",
      "role",
      "ref"
    ]
  },
  {
    "kind": "ui",
    "slug": "button",
    "name": "Button",
    "section": "Primitives",
    "description": "An accessible press target that you style and compose with text or other content.",
    "parts": [],
    "keyProps": [
      "style",
      "children",
      "onClick",
      "disabled"
    ]
  },
  {
    "kind": "ui",
    "slug": "input",
    "name": "Input",
    "section": "Primitives",
    "description": "A controlled, core-owned text editor with native keyboard and text services.",
    "parts": [],
    "keyProps": [
      "value",
      "placeholder",
      "multiline",
      "onInput",
      "onChange"
    ]
  },
  {
    "kind": "ui",
    "slug": "text-area",
    "name": "TextArea",
    "section": "Primitives",
    "description": "The multiline text-editing primitive, with the same controlled value contract as Input.",
    "parts": [],
    "keyProps": [
      "value",
      "placeholder",
      "onInput",
      "onChange",
      "style"
    ]
  },
  {
    "kind": "ui",
    "slug": "markdown",
    "name": "Markdown",
    "section": "Primitives",
    "description": "Renders retained Markdown, including an incremental mode for streaming content.",
    "parts": [],
    "keyProps": [
      "content",
      "source",
      "streaming",
      "style"
    ]
  },
  {
    "kind": "ui",
    "slug": "image",
    "name": "Image",
    "section": "Primitives",
    "description": "Displays a filesystem path, file URL, or base64 data URL with retained image resources.",
    "parts": [],
    "keyProps": [
      "source",
      "fit",
      "objectFit",
      "style"
    ]
  },
  {
    "kind": "ui",
    "slug": "svg",
    "name": "Svg",
    "section": "Primitives",
    "description": "Renders a complete inline SVG document without loading external SVG resources.",
    "parts": [],
    "keyProps": [
      "source",
      "style"
    ]
  },
  {
    "kind": "ui",
    "slug": "shader",
    "name": "Shader",
    "section": "Primitives",
    "description": "Paints validated WGSL with a bounded set of numeric shader parameters.",
    "parts": [],
    "keyProps": [
      "source",
      "shaderParameters",
      "style"
    ]
  },
  {
    "kind": "ui",
    "slug": "virtual-list",
    "name": "VirtualList",
    "section": "Primitives",
    "description": "Mounts only the visible slice of a long or variable-height collection.",
    "parts": [],
    "keyProps": [
      "estimatedItemHeight",
      "overscan",
      "listAlignment",
      "followMode",
      "children"
    ]
  },
  {
    "kind": "ui",
    "slug": "terminal",
    "name": "Terminal",
    "section": "Primitives",
    "description": "Embeds the retained Ghostty terminal surface and reports process lifecycle events.",
    "parts": [],
    "keyProps": [
      "program",
      "arguments",
      "workingDirectory",
      "environment",
      "onStatus"
    ]
  },
  {
    "kind": "ui",
    "slug": "checkbox",
    "name": "Checkbox",
    "section": "Forms & Controls",
    "description": "A binary or indeterminate choice with a separately styled indicator.",
    "parts": [
      "Root",
      "Indicator"
    ],
    "keyProps": [
      "checked",
      "defaultChecked",
      "onCheckedChange",
      "readOnly",
      "disabled"
    ]
  },
  {
    "kind": "ui",
    "slug": "checkbox-group",
    "name": "CheckboxGroup",
    "section": "Forms & Controls",
    "description": "Coordinates a bounded set of checkbox values and an optional derived parent checkbox.",
    "parts": [
      "Root"
    ],
    "keyProps": [
      "allValues",
      "value",
      "defaultValue",
      "onValueChange",
      "disabled"
    ]
  },
  {
    "kind": "ui",
    "slug": "radio",
    "name": "Radio",
    "section": "Forms & Controls",
    "description": "One selectable item, normally declared inside a RadioGroup.",
    "parts": [
      "Root",
      "Indicator"
    ],
    "keyProps": [
      "value",
      "checked",
      "onCheckedChange",
      "disabled"
    ]
  },
  {
    "kind": "ui",
    "slug": "radio-group",
    "name": "RadioGroup",
    "section": "Forms & Controls",
    "description": "Owns single-choice selection and keyboard movement across Radio items.",
    "parts": [
      "Root"
    ],
    "keyProps": [
      "value",
      "defaultValue",
      "onValueChange",
      "readOnly",
      "required"
    ]
  },
  {
    "kind": "ui",
    "slug": "switch",
    "name": "Switch",
    "section": "Forms & Controls",
    "description": "An on/off control with a root track and independently styled thumb.",
    "parts": [
      "Root",
      "Thumb"
    ],
    "keyProps": [
      "checked",
      "defaultChecked",
      "onCheckedChange",
      "disabled"
    ]
  },
  {
    "kind": "ui",
    "slug": "toggle",
    "name": "Toggle",
    "section": "Forms & Controls",
    "description": "A pressable control that retains an on or off pressed state.",
    "parts": [
      "Root",
      "Indicator"
    ],
    "keyProps": [
      "pressed",
      "defaultPressed",
      "onPressedChange",
      "disabled"
    ]
  },
  {
    "kind": "ui",
    "slug": "toggle-group",
    "name": "ToggleGroup",
    "section": "Forms & Controls",
    "description": "Coordinates single or multiple pressed values with roving keyboard focus.",
    "parts": [
      "Root",
      "Item"
    ],
    "keyProps": [
      "items",
      "value",
      "defaultValue",
      "variant",
      "onValueChange"
    ]
  },
  {
    "kind": "ui",
    "slug": "slider",
    "name": "Slider",
    "section": "Forms & Controls",
    "description": "A single- or multi-thumb range input with core-owned pointer and keyboard interaction.",
    "parts": [
      "Root",
      "Label",
      "Value",
      "Control",
      "Track",
      "Range",
      "Indicator",
      "Thumb"
    ],
    "keyProps": [
      "value",
      "defaultValue",
      "min",
      "max",
      "step",
      "onValueChange"
    ]
  },
  {
    "kind": "ui",
    "slug": "number-field",
    "name": "NumberField",
    "section": "Forms & Controls",
    "description": "A numeric editor with increment, decrement, and drag-to-scrub parts.",
    "parts": [
      "Root",
      "Group",
      "Input",
      "Increment",
      "Decrement",
      "ScrubArea",
      "ScrubAreaCursor"
    ],
    "keyProps": [
      "value",
      "defaultValue",
      "min",
      "max",
      "step",
      "onValueChange"
    ]
  },
  {
    "kind": "ui",
    "slug": "select",
    "name": "Select",
    "section": "Forms & Controls",
    "description": "A single- or multiple-choice picker whose option surface is rendered in a native popover window.",
    "parts": [
      "Root",
      "Trigger",
      "Option",
      "Label",
      "Value",
      "Icon",
      "Backdrop",
      "Portal",
      "Positioner",
      "Popup",
      "Arrow",
      "List",
      "Item",
      "ItemText",
      "ItemIndicator",
      "Group",
      "GroupLabel",
      "Separator",
      "ScrollUpArrow",
      "ScrollDownArrow"
    ],
    "keyProps": [
      "items",
      "value",
      "defaultValue",
      "multiple",
      "onValueChange"
    ]
  },
  {
    "kind": "ui",
    "slug": "combobox",
    "name": "Combobox",
    "section": "Forms & Controls",
    "description": "Combines editable text, filtering, optional chips, and a native suggestion surface.",
    "parts": [
      "Root",
      "Input",
      "Option",
      "Label",
      "Value",
      "Icon",
      "InputGroup",
      "Clear",
      "Trigger",
      "Chips",
      "Chip",
      "ChipRemove",
      "Backdrop",
      "Portal",
      "Positioner",
      "Popup",
      "Arrow",
      "Status",
      "Empty",
      "List",
      "Row",
      "Item",
      "ItemIndicator",
      "Group",
      "GroupLabel",
      "Collection",
      "Separator"
    ],
    "keyProps": [
      "items",
      "value",
      "inputValue",
      "multiple",
      "filterMode",
      "onValueChange"
    ]
  },
  {
    "kind": "ui",
    "slug": "autocomplete",
    "name": "Autocomplete",
    "section": "Forms & Controls",
    "description": "Filters suggestions for free-form text without requiring a selected value.",
    "parts": [
      "Root",
      "Input",
      "Option",
      "Label",
      "Value",
      "Icon",
      "InputGroup",
      "Clear",
      "Portal",
      "Positioner",
      "Popup",
      "Arrow",
      "Status",
      "Empty",
      "List",
      "Item",
      "ItemIndicator",
      "Group",
      "GroupLabel",
      "Collection",
      "Separator"
    ],
    "keyProps": [
      "items",
      "inputValue",
      "filterMode",
      "onInputValueChange",
      "onCommit"
    ]
  },
  {
    "kind": "ui",
    "slug": "field",
    "name": "Field",
    "section": "Forms & Controls",
    "description": "Connects one control to its label, description, validation state, and error message.",
    "parts": [
      "Root",
      "Item",
      "Label",
      "Control",
      "Validity",
      "Description",
      "Error"
    ],
    "keyProps": [
      "invalid",
      "required",
      "validationMessage",
      "validationMode",
      "onValidationChange"
    ]
  },
  {
    "kind": "ui",
    "slug": "fieldset",
    "name": "Fieldset",
    "section": "Forms & Controls",
    "description": "Groups related fields under one legend and shared semantic state.",
    "parts": [
      "Root",
      "Legend",
      "Description",
      "Control"
    ],
    "keyProps": [
      "disabled",
      "children",
      "style"
    ]
  },
  {
    "kind": "ui",
    "slug": "date-field",
    "name": "DateField",
    "section": "Forms & Controls",
    "description": "A segmented, keyboard-editable date field with locale-aware date parts.",
    "parts": [
      "Root",
      "Segment"
    ],
    "keyProps": [
      "value",
      "min",
      "max",
      "onValueChange",
      "disabled"
    ]
  },
  {
    "kind": "ui",
    "slug": "time-field",
    "name": "TimeField",
    "section": "Forms & Controls",
    "description": "A segmented, keyboard-editable time field with hour, minute, second, and period parts.",
    "parts": [
      "Root",
      "Segment"
    ],
    "keyProps": [
      "value",
      "defaultValue",
      "hour12",
      "showSeconds",
      "onValueChange"
    ]
  },
  {
    "kind": "ui",
    "slug": "calendar",
    "name": "Calendar",
    "section": "Forms & Controls",
    "description": "A keyboard-navigable month grid for single-date selection.",
    "parts": [
      "Root",
      "Week",
      "Day"
    ],
    "keyProps": [
      "value",
      "defaultValue",
      "min",
      "max",
      "firstWeekday",
      "onValueChange"
    ]
  },
  {
    "kind": "ui",
    "slug": "otp-field",
    "name": "OtpField",
    "section": "Forms & Controls",
    "description": "A bounded one-time-code editor made from individually styled input cells.",
    "parts": [
      "Root",
      "Input",
      "Separator"
    ],
    "keyProps": [
      "value",
      "length",
      "onValueChange",
      "disabled"
    ]
  },
  {
    "kind": "ui",
    "slug": "tabs",
    "name": "Tabs",
    "section": "Layout & Data",
    "description": "Switches between labeled panels with automatic or manual keyboard activation.",
    "parts": [
      "Root",
      "List",
      "Tab",
      "Indicator",
      "Panel"
    ],
    "keyProps": [
      "value",
      "defaultValue",
      "onValueChange",
      "orientation",
      "activation"
    ]
  },
  {
    "kind": "ui",
    "slug": "accordion",
    "name": "Accordion",
    "section": "Layout & Data",
    "description": "Coordinates one or more expandable sections and their keyboard focus.",
    "parts": [
      "Root",
      "Item",
      "Header",
      "Trigger",
      "Panel"
    ],
    "keyProps": [
      "value",
      "defaultValue",
      "onValueChange",
      "multiple",
      "keepMounted"
    ]
  },
  {
    "kind": "ui",
    "slug": "collapsible",
    "name": "Collapsible",
    "section": "Layout & Data",
    "description": "Shows and hides one panel from a trigger while preserving accessible state.",
    "parts": [
      "Root",
      "Trigger",
      "Panel"
    ],
    "keyProps": [
      "open",
      "defaultOpen",
      "onOpenChange",
      "keepMounted"
    ]
  },
  {
    "kind": "ui",
    "slug": "splitter",
    "name": "Splitter",
    "section": "Layout & Data",
    "description": "Creates resizable panes with pointer and keyboard-operated handles.",
    "parts": [
      "Root",
      "Pane",
      "Handle"
    ],
    "keyProps": [
      "value",
      "defaultValue",
      "panes",
      "orientation",
      "onSizesChange"
    ]
  },
  {
    "kind": "ui",
    "slug": "scroll-area",
    "name": "ScrollArea",
    "section": "Layout & Data",
    "description": "Composes a scrollable viewport with caller-styled scrollbars, thumbs, and corner.",
    "parts": [
      "Root",
      "Viewport",
      "Content",
      "Scrollbar",
      "Thumb",
      "Corner"
    ],
    "keyProps": [
      "viewportSize",
      "contentSize",
      "overflowEdgeThreshold",
      "onScrollStateChange"
    ]
  },
  {
    "kind": "ui",
    "slug": "table",
    "name": "Table",
    "section": "Layout & Data",
    "description": "A virtualized tabular collection with header, row, and cell parts.",
    "parts": [
      "Root",
      "Header",
      "Row",
      "Cell"
    ],
    "keyProps": [
      "columns",
      "rowCount",
      "rowHeight",
      "selectionMode",
      "selection"
    ]
  },
  {
    "kind": "ui",
    "slug": "tree",
    "name": "Tree",
    "section": "Layout & Data",
    "description": "A virtualized hierarchical collection with expansion, selection, and keyboard navigation.",
    "parts": [
      "Root",
      "Row"
    ],
    "keyProps": [
      "nodes",
      "expanded",
      "value",
      "onExpandedChange",
      "onValueChange"
    ]
  },
  {
    "kind": "ui",
    "slug": "separator",
    "name": "Separator",
    "section": "Layout & Data",
    "description": "A semantic horizontal or vertical divider with no built-in visual style.",
    "parts": [
      "Root"
    ],
    "keyProps": [
      "orientation",
      "style"
    ]
  },
  {
    "kind": "ui",
    "slug": "avatar",
    "name": "Avatar",
    "section": "Layout & Data",
    "description": "Shows an image with a fallback that appears when the image is unavailable.",
    "parts": [
      "Root",
      "Image",
      "Fallback"
    ],
    "keyProps": [
      "ariaLabel",
      "src",
      "delay",
      "onLoadingStatusChange"
    ]
  },
  {
    "kind": "ui",
    "slug": "progress",
    "name": "Progress",
    "section": "Layout & Data",
    "description": "Represents determinate or indeterminate task progress with composable labels and track.",
    "parts": [
      "Root",
      "Track",
      "Indicator",
      "Label",
      "Value"
    ],
    "keyProps": [
      "value",
      "max",
      "indeterminate",
      "valueText",
      "onStatusChange"
    ]
  },
  {
    "kind": "ui",
    "slug": "meter",
    "name": "Meter",
    "section": "Layout & Data",
    "description": "Displays a scalar measurement within a known range and semantic low, high, and optimum bands.",
    "parts": [
      "Root",
      "Track",
      "Indicator",
      "Label",
      "Value"
    ],
    "keyProps": [
      "value",
      "min",
      "max",
      "low",
      "high",
      "optimum"
    ]
  },
  {
    "kind": "ui",
    "slug": "toolbar",
    "name": "Toolbar",
    "section": "Layout & Data",
    "description": "Groups actions and inputs under one roving-focus keyboard model.",
    "parts": [
      "Root",
      "Item",
      "Button",
      "Link",
      "Input",
      "Group",
      "Separator"
    ],
    "keyProps": [
      "items",
      "active",
      "defaultActive",
      "orientation",
      "onActiveChange"
    ]
  },
  {
    "kind": "ui",
    "slug": "popover",
    "name": "Popover",
    "section": "Overlays",
    "description": "Places caller-styled content beside an anchor inside the current window.",
    "parts": [
      "Root",
      "Trigger",
      "Content",
      "Portal",
      "Backdrop",
      "Positioner",
      "Popup",
      "Arrow",
      "Viewport",
      "Title",
      "Description",
      "Close"
    ],
    "keyProps": [
      "open",
      "defaultOpen",
      "onOpenChange",
      "side",
      "align",
      "modal"
    ]
  },
  {
    "kind": "ui",
    "slug": "system-popover",
    "name": "SystemPopover",
    "section": "Overlays",
    "description": "Presents QuickGUI content in a separate native child window that can extend beyond its owner.",
    "parts": [
      "Root",
      "Trigger",
      "Content"
    ],
    "keyProps": [
      "open",
      "defaultOpen",
      "onOpenChange",
      "width",
      "height",
      "placement"
    ]
  },
  {
    "kind": "ui",
    "slug": "dialog",
    "name": "Dialog",
    "section": "Overlays",
    "description": "An in-window modal with focus containment, dismissal, and exact transition completion.",
    "parts": [
      "Root",
      "Trigger",
      "Portal",
      "Backdrop",
      "Viewport",
      "Popup",
      "Title",
      "Description",
      "Close"
    ],
    "keyProps": [
      "open",
      "defaultOpen",
      "onOpenChange",
      "dismissOnEscape",
      "exitDuration"
    ]
  },
  {
    "kind": "ui",
    "slug": "alert-dialog",
    "name": "AlertDialog",
    "section": "Overlays",
    "description": "The confirmation-focused dialog variant with stricter backdrop-dismissal defaults.",
    "parts": [
      "Root",
      "Trigger",
      "Portal",
      "Backdrop",
      "Viewport",
      "Popup",
      "Title",
      "Description",
      "Close"
    ],
    "keyProps": [
      "open",
      "defaultOpen",
      "onOpenChange",
      "dismissOnEscape",
      "exitDuration"
    ]
  },
  {
    "kind": "ui",
    "slug": "tooltip",
    "name": "Tooltip",
    "section": "Overlays",
    "description": "Shows passive help after an exact core-owned delay and supports provider-level warm-up behavior.",
    "parts": [
      "Provider",
      "Root",
      "Trigger",
      "Portal",
      "Positioner",
      "Popup",
      "Arrow"
    ],
    "keyProps": [
      "open",
      "defaultOpen",
      "delay",
      "closeDelay",
      "side",
      "hoverable"
    ]
  },
  {
    "kind": "ui",
    "slug": "preview-card",
    "name": "PreviewCard",
    "section": "Overlays",
    "description": "Shows a richer hover or focus preview beside a trigger without changing the current screen.",
    "parts": [
      "Root",
      "Trigger",
      "Portal",
      "Backdrop",
      "Positioner",
      "Popup",
      "Arrow"
    ],
    "keyProps": [
      "open",
      "defaultOpen",
      "delay",
      "closeDelay",
      "side"
    ]
  },
  {
    "kind": "ui",
    "slug": "toast",
    "name": "Toast",
    "section": "Overlays",
    "description": "Presents timed, stacked notifications with action, close, and swipe behavior.",
    "parts": [
      "Provider",
      "Portal",
      "Viewport",
      "Positioner",
      "Root",
      "Content",
      "Title",
      "Description",
      "Action",
      "Close"
    ],
    "keyProps": [
      "toasts",
      "toastId",
      "timeout",
      "swipeDirection",
      "onDismiss"
    ]
  },
  {
    "kind": "ui",
    "slug": "menu",
    "name": "Menu",
    "section": "Menus & Navigation",
    "description": "A fully composable menu with submenus, groups, links, checkbox items, and radio items.",
    "parts": [
      "Root",
      "Trigger",
      "Portal",
      "Backdrop",
      "Positioner",
      "Popup",
      "Arrow",
      "Item",
      "LinkItem",
      "SubmenuRoot",
      "SubmenuTrigger",
      "Group",
      "GroupLabel",
      "RadioGroup",
      "RadioItem",
      "RadioItemIndicator",
      "CheckboxItem",
      "CheckboxItemIndicator",
      "Separator"
    ],
    "keyProps": [
      "open",
      "defaultOpen",
      "onOpenChange",
      "orientation",
      "loopFocus"
    ]
  },
  {
    "kind": "ui",
    "slug": "popover-menu",
    "name": "PopoverMenu",
    "section": "Menus & Navigation",
    "description": "A compact model-driven menu rendered in a native popover window.",
    "parts": [
      "Root",
      "Trigger",
      "Popup"
    ],
    "keyProps": [
      "items",
      "appearance",
      "open",
      "onOpenChange",
      "onSelect"
    ]
  },
  {
    "kind": "ui",
    "slug": "context-menu",
    "name": "ContextMenu",
    "section": "Menus & Navigation",
    "description": "Opens a model-driven native menu from a secondary-button press on its trigger.",
    "parts": [
      "Root",
      "Trigger"
    ],
    "keyProps": [
      "items",
      "appearance",
      "onSelect",
      "loop"
    ]
  },
  {
    "kind": "ui",
    "slug": "menubar",
    "name": "Menubar",
    "section": "Menus & Navigation",
    "description": "Coordinates a horizontal in-window menubar and its menu items.",
    "parts": [
      "Root",
      "Item"
    ],
    "keyProps": [
      "count",
      "open",
      "defaultOpen",
      "onOpenChange",
      "onActiveChange"
    ]
  },
  {
    "kind": "ui",
    "slug": "navigation-menu",
    "name": "NavigationMenu",
    "section": "Menus & Navigation",
    "description": "A disclosure-style navigation surface with animated direction, viewport, and popup parts.",
    "parts": [
      "Root",
      "List",
      "Item",
      "Trigger",
      "Icon",
      "Content",
      "Link",
      "Portal",
      "Positioner",
      "Popup",
      "Viewport",
      "Arrow",
      "Backdrop"
    ],
    "keyProps": [
      "value",
      "defaultValue",
      "onValueChange",
      "orientation",
      "delay"
    ]
  },
  {
    "kind": "ui",
    "slug": "router",
    "name": "Router",
    "section": "Menus & Navigation",
    "description": "A QuickGUI UI projection of QuickGUI core routing with nested layouts and bounded memory history.",
    "parts": [
      "Router",
      "route",
      "layout",
      "Link",
      "Outlet"
    ],
    "keyProps": [
      "initialPath",
      "fallback",
      "routes",
      "path",
      "component",
      "href"
    ]
  },
  {
    "kind": "swift-ui",
    "slug": "host",
    "name": "Host",
    "section": "SwiftUI Components",
    "description": "The NSHostingView-backed leaf that mounts one SwiftUI component tree inside QuickGUI.",
    "parts": [],
    "keyProps": [
      "matchContents",
      "style",
      "children"
    ]
  },
  {
    "kind": "swift-ui",
    "slug": "button",
    "name": "Button",
    "section": "SwiftUI Components",
    "description": "A native SwiftUI button with text, SF Symbol, semantic role, and press handling.",
    "parts": [],
    "keyProps": [
      "label",
      "systemImage",
      "role",
      "onPress",
      "modifiers"
    ]
  },
  {
    "kind": "swift-ui",
    "slug": "slider",
    "name": "Slider",
    "section": "SwiftUI Components",
    "description": "A controlled native slider with continuous or stepped values.",
    "parts": [],
    "keyProps": [
      "value",
      "min",
      "max",
      "step",
      "label",
      "onValueChange"
    ]
  },
  {
    "kind": "swift-ui",
    "slug": "toggle",
    "name": "Toggle",
    "section": "SwiftUI Components",
    "description": "A controlled native SwiftUI on/off switch.",
    "parts": [],
    "keyProps": [
      "isOn",
      "label",
      "onIsOnChange",
      "modifiers"
    ]
  },
  {
    "kind": "swift-ui",
    "slug": "progress-view",
    "name": "ProgressView",
    "section": "SwiftUI Components",
    "description": "A determinate or indeterminate native SwiftUI progress indicator.",
    "parts": [],
    "keyProps": [
      "value",
      "total",
      "label",
      "currentValueLabel"
    ]
  },
  {
    "kind": "swift-ui",
    "slug": "stepper",
    "name": "Stepper",
    "section": "SwiftUI Components",
    "description": "A controlled native numeric stepper with bounded values.",
    "parts": [],
    "keyProps": [
      "value",
      "min",
      "max",
      "step",
      "label",
      "onValueChange"
    ]
  },
  {
    "kind": "swift-ui",
    "slug": "text-field",
    "name": "TextField",
    "section": "SwiftUI Components",
    "description": "A controlled native SwiftUI text field with edit and submit callbacks.",
    "parts": [],
    "keyProps": [
      "value",
      "placeholder",
      "onValueChange",
      "onSubmit"
    ]
  },
  {
    "kind": "swift-ui",
    "slug": "secure-field",
    "name": "SecureField",
    "section": "SwiftUI Components",
    "description": "The concealed native editor with the same controlled contract as TextField.",
    "parts": [],
    "keyProps": [
      "value",
      "placeholder",
      "onValueChange",
      "onSubmit"
    ]
  },
  {
    "kind": "swift-ui",
    "slug": "picker",
    "name": "Picker",
    "section": "SwiftUI Components",
    "description": "A controlled SwiftUI picker with menu, segmented, radio-group, and inline styles.",
    "parts": [],
    "keyProps": [
      "selection",
      "options",
      "label",
      "style",
      "onSelectionChange"
    ]
  },
  {
    "kind": "swift-ui",
    "slug": "segmented-control",
    "name": "SegmentedControl",
    "section": "SwiftUI Components",
    "description": "A native segmented picker with value-selection and neutral Xcode-style tab roles.",
    "parts": [],
    "keyProps": [
      "selection",
      "options",
      "label",
      "role",
      "onSelectionChange"
    ]
  },
  {
    "kind": "swift-ui",
    "slug": "date-picker",
    "name": "DatePicker",
    "section": "SwiftUI Components",
    "description": "A controlled native date or date-and-time picker.",
    "parts": [],
    "keyProps": [
      "value",
      "min",
      "max",
      "displayedComponents",
      "style",
      "onValueChange"
    ]
  },
  {
    "kind": "swift-ui",
    "slug": "color-picker",
    "name": "ColorPicker",
    "section": "SwiftUI Components",
    "description": "A native SwiftUI color well that returns CSS-style RGBA color strings.",
    "parts": [],
    "keyProps": [
      "selection",
      "label",
      "supportsOpacity",
      "onSelectionChange"
    ]
  },
  {
    "kind": "swift-ui",
    "slug": "gauge",
    "name": "Gauge",
    "section": "SwiftUI Components",
    "description": "A native SwiftUI gauge with linear and circular accessory styles.",
    "parts": [],
    "keyProps": [
      "value",
      "min",
      "max",
      "label",
      "currentValueLabel",
      "style"
    ]
  },
  {
    "kind": "swift-ui",
    "slug": "quickgui-host-view",
    "name": "QuickGUIHostView",
    "section": "SwiftUI Components",
    "description": "Reverse-hosts one ordinary QuickGUI subtree inside a SwiftUI hierarchy.",
    "parts": [],
    "keyProps": [
      "width",
      "height",
      "matchContents",
      "background",
      "children"
    ]
  },
  {
    "kind": "swift-ui",
    "slug": "popover",
    "name": "Popover",
    "section": "SwiftUI Components",
    "description": "Presents a native SwiftUI popover from a composed SwiftUI trigger.",
    "parts": [
      "Trigger",
      "Content"
    ],
    "keyProps": [
      "isPresented",
      "onIsPresentedChange",
      "attachmentAnchor",
      "arrowEdge"
    ]
  }
] as const satisfies readonly ComponentDoc[]

export const UI_COMPONENTS: readonly ComponentDoc[] =
  ALL_COMPONENT_DOCS.filter((component) => component.kind === 'ui')

export const SWIFT_UI_COMPONENTS: readonly ComponentDoc[] =
  ALL_COMPONENT_DOCS.filter((component) => component.kind === 'swift-ui')

export const COMPONENT_NAV_GROUPS: readonly ComponentNavGroup[] = [
  'Primitives',
  'Forms & Controls',
  'Layout & Data',
  'Overlays',
  'Menus & Navigation',
].map((title) => ({
  title: title as ComponentDocSection,
  items: UI_COMPONENTS.filter((component) => component.section === title),
}))

export const SWIFT_UI_NAV_GROUP: ComponentNavGroup = {
  title: 'SwiftUI Components',
  items: SWIFT_UI_COMPONENTS,
}

export function componentDocsPath(component: ComponentDoc): string {
  const section = component.kind === 'swift-ui' ? 'swift-ui' : 'components'
  return `/docs/${section}/${component.slug}`
}

export function findComponentDoc(
  kind: ComponentDocKind,
  slug?: string,
): ComponentDoc | undefined {
  return ALL_COMPONENT_DOCS.find(
    (component) => component.kind === kind && component.slug === slug,
  )
}

export function componentOutline(component: ComponentDoc): readonly DocsOutlineItem[] {
  return [
    { id: 'import', title: 'Import' },
    { id: 'usage', title: 'Usage' },
    ...(component.parts.length
      ? [{ id: 'anatomy', title: 'Anatomy' } as const]
      : []),
    { id: 'key-props', title: 'Key props' },
  ]
}
