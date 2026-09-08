# Components · MoonBit

The MoonBit counterpart of [`../components`](../components), with all 43 demos
and the same sidebar, selection styling, window layout, and system appearance.
Everything in the gallery uses QuickGUI components; the System Context Menu
demo intentionally opens an operating-system menu.

From the repository root:

```sh
bun install
bun scripts/setup-moonbit.ts # pinned compiler for this checkout
export MOON_HOME="$PWD/target/moonbit-toolchain"
export PATH="$MOON_HOME/bin:$PATH"
bun run build:native        # once, unless the host library is already built
bun --cwd examples/components-moonbit dev
```

`bun --cwd examples/components-moonbit build` packages the application. Ordinary
MoonBit edits reuse the native library and use the CLI's fast development backend.

## Demos

| Source | Demos |
| --- | --- |
| [`gallery/basics.mbt`](gallery/basics.mbt) | Accordion, Avatar, Button, Checkbox, Checkbox Group, Collapsible |
| [`gallery/controls.mbt`](gallery/controls.mbt) | Meter, Progress, Radio, Separator, Slider, Splitter, Switch, Tabs, Toggle, Toggle Group, Toolbar |
| [`gallery/forms.mbt`](gallery/forms.mbt) | Calendar, Date Field, Field, Fieldset, Form, Input, Number Field, OTP Field, Time Field |
| [`gallery/pickers.mbt`](gallery/pickers.mbt) | Autocomplete, Combobox, Select |
| [`gallery/overlays.mbt`](gallery/overlays.mbt) | Alert Dialog, Context Menu, System Context Menu, Dialog, Menu, Menubar, Navigation Menu, Popover, Preview Card, Toast, Tooltip |
| [`gallery/data.mbt`](gallery/data.mbt) | Scroll Area, Table, Tree |

The demos include mixed and read-only checkboxes, grouped selection, animated
switches, range sliders, keyboard navigation, validation, editable date/time
segments, multiple pickers and removable chips, nested menus, dialogs and focus
restoration, toast deadlines, a 5,000-row virtual table, and lazy tree children.
Only the selected demo is mounted. Sidebar identities survive navigation, and
retiring a panel disposes its nodes and reactive owners. Table sorting is memoized
separately from scrolling; only visible rows are constructed.

## Checks

```sh
bun --cwd examples/components-moonbit check
# With a staged host library and a native display/session:
bun --cwd examples/components-moonbit check --native
```

The default checks run the SDK and gallery tests through both development and
production backends. They cover retained sidebar selection, theme updates,
checkbox state, bounded table rows and sorting, lazy tree children, and calendar
identities. `--native` additionally presents every demo in light and dark colors
in self-closing hidden windows and verifies toast deadlines and owner cleanup. This exercises native
layout/rendering; keyboard, pointer, and platform appearance acceptance can be
checked in the interactive gallery.
