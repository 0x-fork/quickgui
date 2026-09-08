// Component identities and configuration forwarded to the existing native core.
// Keep frontend state here limited to declared values and controlled callbacks.
export interface MoonComponent {
  name: string
  root?: string
  tag?: string
  parts: Record<string, string>
  properties: string[]
  defaults?: Record<string, boolean | number | string | unknown[]>
}

const flags = 'DISABLED READ_ONLY REQUIRED'
const selection = `${flags} ACTIVE_VALUE VALUES ITEMS MULTIPLE ORIENTATION LOOP_FOCUS`
const overlay =
  'OPEN DISABLED SIDE ALIGN SIDE_OFFSET ALIGN_OFFSET COLLISION_PADDING STICKY ANCHOR_TARGET ANCHOR_POINT MODAL OPEN_ON_HOVER DELAY CLOSE_DELAY ENTER_DURATION EXIT_DURATION DISMISS_ON_ESCAPE DISMISS_ON_POINTER_OUTSIDE KEEP_MOUNTED'
const fields = `${flags} INVALID TOUCHED DIRTY FILLED VALIDATION_MESSAGE VALIDATION_MODE VALIDATION_DEBOUNCE_TIME`
const picker = `${selection} INPUT_VALUE FILTER_MODE OPTIONS APPEARANCE AUTO_HIGHLIGHT OPEN_ON_INPUT_CLICK HIGHLIGHT_ITEM_ON_HOVER ALIGN_ITEM_WITH_TRIGGER MODAL`
const parts = (prefix: string, names: string) =>
  Object.fromEntries(
    names
      .split(' ')
      .filter(Boolean)
      .map((name) => [name, `${prefix}-${name}`]),
  )
const component = (
  name: string,
  names: string,
  properties: string,
  options: Partial<MoonComponent> = {},
): MoonComponent => ({
  name,
  root: name,
  parts: parts(name, names),
  properties: properties.split(' ').filter(Boolean),
  ...options,
})

export const moonComponents: MoonComponent[] = [
  component('checkbox', 'indicator', `${flags} CHECKED INDETERMINATE PARENT VALUES`, {
    tag: 'BUTTON',
    defaults: { CHECKED: false, INDETERMINATE: false },
  }),
  component('checkbox-group', 'item indicator parent', `${selection}`, {
    defaults: { VALUES: [] },
  }),
  component('radio', 'indicator', `${flags} CHECKED`, {
    tag: 'BUTTON',
    defaults: { CHECKED: false },
  }),
  component('radio-group', '', selection, {
    parts: { item: 'radio', indicator: 'radio-indicator' },
  }),
  component('switch', 'thumb', `${flags} CHECKED`, { tag: 'BUTTON', defaults: { CHECKED: false } }),
  component('toggle', 'indicator', `${flags} PRESSED`, {
    tag: 'BUTTON',
    defaults: { PRESSED: false },
  }),
  component('toggle-group', 'item', selection, { defaults: { VALUES: [] } }),
  component('tabs', 'list', `${selection} ACTIVATE_ON_FOCUS KEEP_MOUNTED`, {
    parts: { list: 'tabs-list', tab: 'tab', indicator: 'tab-indicator', panel: 'tab-panel' },
    defaults: { LOOP_FOCUS: true },
  }),
  component('collapsible', 'trigger panel', 'OPEN DISABLED KEEP_MOUNTED', {
    defaults: { OPEN: false },
  }),
  component(
    'accordion',
    'item header trigger panel',
    'VALUES MULTIPLE DISABLED KEEP_MOUNTED HEADING_LEVEL OPEN',
    { defaults: { VALUES: [], HEADING_LEVEL: 3 } },
  ),
  component(
    'slider',
    'label value control track range indicator thumb',
    `${flags} VALUES MINIMUM MAXIMUM STEP LARGE_STEP MIN_STEPS_BETWEEN_VALUES THUMB_ALIGNMENT FORMAT ORIENTATION`,
    { defaults: { VALUES: [0], MINIMUM: 0, MAXIMUM: 100, STEP: 1 } },
  ),
  component(
    'number-field',
    'group input increment decrement scrub-area scrub-area-cursor',
    `${flags} VALUES MINIMUM MAXIMUM STEP LARGE_STEP SMALL_STEP FORMAT PITCH ALLOW_WHEEL_SCRUB SNAP_ON_STEP`,
    { defaults: { VALUES: [0] } },
  ),
  component(
    'select',
    'label value icon backdrop portal positioner popup arrow list item item-text item-indicator group group-label separator scroll-up-arrow scroll-down-arrow',
    picker,
    { tag: 'BUTTON' },
  ),
  component(
    'combobox',
    'label value icon input-group clear trigger chips chip chip-remove backdrop portal positioner popup arrow status empty list row item item-indicator group group-label collection separator',
    picker,
    { tag: 'INPUT' },
  ),
  component('autocomplete', '', picker, {
    tag: 'INPUT',
    parts: parts(
      'combobox',
      'label value icon input-group clear portal positioner popup arrow status empty list item item-indicator group group-label collection separator',
    ),
  }),
  component('field', 'item label control validity description error', fields),
  component('fieldset', 'legend description control', 'DISABLED'),
  component(
    'date-field',
    'segment',
    `${flags} CIVIL_VALUE CIVIL_MINIMUM CIVIL_MAXIMUM SEGMENT_ORDER`,
  ),
  component(
    'time-field',
    'segment',
    `${flags} CIVIL_VALUE CIVIL_MINIMUM CIVIL_MAXIMUM SEGMENT_ORDER VARIANT FORMAT`,
  ),
  component(
    'calendar',
    'week day',
    `${flags} CIVIL_VALUE CIVIL_MINIMUM CIVIL_MAXIMUM FIRST_WEEKDAY`,
  ),
  component('otp-field', 'input separator', `${flags} VALUE LENGTH MASK AUTO_SUBMIT`, {
    defaults: { LENGTH: 6 },
  }),
  component('splitter', 'pane handle', 'VALUES ITEMS ORIENTATION', {
    defaults: { VALUES: [240, 480] },
  }),
  component(
    'scroll-area',
    'viewport content scrollbar thumb corner',
    'VALUES VIEWPORT_SIZE CONTENT_SIZE OVERFLOW_EDGE_THRESHOLD ORIENTATION',
  ),
  component(
    'table',
    'header row cell',
    'COLUMNS ROW_COUNT ROW_HEIGHT HEADER_HEIGHT SELECTION_MODE SELECTION SORT_COLUMN SORT_DIRECTION',
  ),
  component(
    'tree',
    'row',
    'NODES EXPANDED SELECTED_VALUE ROW_HEIGHT HEADER_HEIGHT SELECTION_MODE SELECTION',
  ),
  component('separator', '', 'ORIENTATION'),
  component('avatar', 'image fallback', 'DELAY'),
  component(
    'progress',
    'track indicator label value',
    'VALUES MAXIMUM INDETERMINATE VALUE_TEXT FORMAT',
    { defaults: { VALUES: [0], MAXIMUM: 1 } },
  ),
  component(
    'meter',
    'track indicator label value',
    'VALUES MINIMUM MAXIMUM LOW HIGH OPTIMUM VALUE_TEXT FORMAT',
    { defaults: { VALUES: [0], MINIMUM: 0, MAXIMUM: 1 } },
  ),
  component('toolbar', 'item button link input group separator', `${selection} ITEM_INDEX`),
  component(
    'dialog',
    'trigger portal backdrop viewport popup title description close',
    `${overlay} VARIANT`,
    {
      root: '',
      defaults: {
        OPEN: false,
        VARIANT: 'dialog',
        DISMISS_ON_ESCAPE: true,
        DISMISS_ON_POINTER_OUTSIDE: true,
      },
      parts: {
        ...parts('dialog', 'trigger backdrop viewport popup title description close'),
        portal: 'dialog',
      },
    },
  ),
  component('alert-dialog', '', `${overlay} VARIANT`, {
    root: '',
    defaults: {
      OPEN: false,
      VARIANT: 'alertdialog',
      DISMISS_ON_ESCAPE: true,
      DISMISS_ON_POINTER_OUTSIDE: false,
    },
    parts: {
      ...parts('dialog', 'trigger backdrop viewport popup title description close'),
      portal: 'dialog',
    },
  }),
  component(
    'popover',
    'trigger portal backdrop positioner popup arrow viewport title description close',
    overlay,
    {
      root: '',
      defaults: {
        OPEN: false,
        SIDE: 'bottom',
        ALIGN: 'start',
        DISMISS_ON_ESCAPE: true,
        DISMISS_ON_POINTER_OUTSIDE: true,
      },
    },
  ),
  component(
    'tooltip',
    'trigger portal positioner popup arrow',
    `${overlay} PROVIDER HOVERABLE TRACK_CURSOR_AXIS CLOSE_ON_CLICK`,
    { root: '', defaults: { DELAY: 600, CLOSE_DELAY: 100 } },
  ),
  component('tooltip-provider', '', 'DELAY CLOSE_DELAY TIMEOUT'),
  component(
    'preview-card',
    'trigger portal backdrop positioner popup arrow',
    `${overlay} DISABLE_POINTER_DISMISSAL`,
    { defaults: { DELAY: 600, CLOSE_DELAY: 300 } },
  ),
  component(
    'drawer',
    'trigger portal backdrop viewport popup content title description close swipe-area',
    `${overlay} SWIPE_DIRECTION`,
  ),
  component('toast-viewport', '', 'TOASTS TIMEOUT LIMIT SWIPE_DIRECTION STACK_EXPANDED', {
    parts: {
      ...parts('toast', 'portal positioner content title description action close'),
      item: 'toast',
    },
  }),
  component(
    'menu',
    'trigger portal backdrop positioner popup arrow item link-item submenu-root submenu-trigger group group-label radio-group radio-item radio-item-indicator checkbox-item checkbox-item-indicator separator',
    `${overlay} ORIENTATION LOOP_FOCUS CLOSE_PARENT_ON_ESC MENU`,
  ),
  component('popover-menu', '', `${overlay} MENU ANCHOR_PLACEMENT ANCHOR_GAP VIEWPORT_MARGIN`, {
    root: '',
    parts: { trigger: 'popover-menu-trigger', popup: 'popover-menu-popup' },
    defaults: { OPEN: false, ANCHOR_PLACEMENT: 'bottom-start', ANCHOR_GAP: 4, VIEWPORT_MARGIN: 8 },
  }),
  component('context-menu', '', 'MENU LOOP_FOCUS', {
    root: '',
    parts: { trigger: 'context-menu-trigger' },
  }),
  component('system-popover', '', 'OPEN DISABLED', {
    root: '',
    tag: 'BUTTON',
    defaults: { OPEN: false },
  }),
  component('menubar', 'item', 'MENU_COUNT OPEN ITEM_INDEX LOOP_FOCUS'),
  component(
    'navigation-menu',
    'list item trigger icon content link portal positioner popup viewport arrow backdrop',
    `${overlay} ACTIVE_VALUE ORIENTATION LOOP_FOCUS`,
  ),
]

// Native property representations. Shared configuration is observed separately
// per property, so a selection change does not invalidate unrelated style.
export const booleanProperties =
  `${flags} CHECKED INDETERMINATE PRESSED PARENT MULTIPLE ACTIVATE_ON_FOCUS LOOP_FOCUS KEEP_MOUNTED OPEN INVALID TOUCHED DIRTY FILLED MODAL OPEN_ON_HOVER STICKY DISMISS_ON_ESCAPE DISMISS_ON_POINTER_OUTSIDE AUTO_HIGHLIGHT OPEN_ON_INPUT_CLICK HIGHLIGHT_ITEM_ON_HOVER ALIGN_ITEM_WITH_TRIGGER HOVERABLE CLOSE_ON_CLICK AUTO_SUBMIT ALLOW_WHEEL_SCRUB SNAP_ON_STEP DISABLE_POINTER_DISMISSAL STACK_EXPANDED CLOSE_PARENT_ON_ESC PASSWORD STREAMING`.split(
    ' ',
  )
export const numberProperties =
  'MINIMUM MAXIMUM LOW HIGH OPTIMUM STEP LARGE_STEP SMALL_STEP MIN_STEPS_BETWEEN_VALUES PITCH HEADING_LEVEL LENGTH ROW_COUNT ROW_HEIGHT HEADER_HEIGHT ITEM_INDEX ROW_INDEX COLUMN_INDEX DELAY CLOSE_DELAY TIMEOUT LIMIT FIRST_WEEKDAY MENU_COUNT SIDE_OFFSET ALIGN_OFFSET COLLISION_PADDING ENTER_DURATION EXIT_DURATION VALIDATION_DEBOUNCE_TIME OVERFLOW_EDGE_THRESHOLD ANCHOR_GAP VIEWPORT_MARGIN OVERSCAN ESTIMATED_ITEM_HEIGHT'.split(
    ' ',
  )
export const jsonProperties =
  'VALUES ITEMS OPTIONS APPEARANCE COLUMNS NODES EXPANDED SELECTION TOASTS VIEWPORT_SIZE CONTENT_SIZE ANCHOR_POINT MENU SHADER_PARAMETERS'.split(
    ' ',
  )
export const extraProperties =
  'HREF PART_VALUE SELECTED_VALUE CIVIL_VALUE CIVIL_MINIMUM CIVIL_MAXIMUM SEGMENT SEGMENT_ORDER GROUP VALUE_TEXT FORMAT ANCHOR_TARGET ANCHOR_PLACEMENT FOLLOW_MODE LIST_ALIGNMENT'.split(
    ' ',
  )
