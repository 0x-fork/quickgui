# Tabs

[Documentation index](README.md) · [Unstyled component roadmap](component-roadmap.md)

QuickGUI provides controlled, unstyled in-window tabs. The application owns the selected value,
all content, layout, typography, colors, borders, focus paint, indicator geometry, and motion.
QuickGUI supplies stable root/list/tab/indicator/panel parts, keyboard navigation, mounting policy,
and native accessibility semantics.

The behavior follows the current [Base UI Tabs](https://base-ui.com/react/components/tabs) and
[WAI-ARIA Tabs Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) contracts. These are GPU
elements in one QuickGUI window; they are independent of the optional native AppKit document-tab
API.

## Controlled composition

`TabsState` is a copy-only convenience value. The owning view may instead store an
`Option<ElementId>` and construct `Tabs::new` or `Tabs::without_selection` directly.

```rust
use quickgui::{Tabs, TabsState, div};

struct Workspace {
    section: TabsState,
}

let tabs = Tabs::from_state("workspace-tabs", &self.section);
let overview = tabs.tab("overview");
let files = tabs.tab("files");

let select_overview = cx.listener(overview.tab_id(), |view, cx| {
    if view.section.select("overview") {
        cx.invalidate();
    }
});
let select_files = cx.listener(files.tab_id(), |view, cx| {
    if view.section.select("files") {
        cx.invalidate();
    }
});

tabs.root_part(
    div()
        .child(tabs.list_part(
            div()
                .child(
                    overview
                        .tab_part(div().child("Overview"))
                        .on_click(select_overview),
                )
                .child(
                    files
                        .tab_part(div().child("Files"))
                        .on_click(select_files),
                ),
        ))
        .children(overview.panel_part(div().child("Workspace activity")))
        .children(files.panel_part(div().child("Project files"))),
)
```

`Tab::state()` exposes `active`, `disabled`, and `orientation` for caller-authored presentation.
`indicator_part` mounts a caller-owned decorative element only for the active tab and hides that
element from accessibility. QuickGUI does not measure or animate a shared indicator; callers can
put an underline inside each tab, as the gallery does, without retaining a geometry observer.

## Keyboard and focus

Horizontal tab lists use Left and Right. Vertical tab lists use Up and Down and leave the other
axis available to the surrounding application. Home and End move to the first and last enabled
tab. Disabled tabs are skipped.

Manual activation is the default: arrows move focus, while Enter or Space invokes the focused
tab's ordinary click listener. `.activate_on_focus(true)` selects through that same listener as
arrow, Home, or End focus moves. `.loop_focus(true)` is the default; set it to `false` to stop at
the first and last enabled tabs.

Only the selected enabled tab, or the first enabled tab when no selected tab is mounted, appears
in the window's normal Tab sequence. Arrow-focused inactive tabs still represent the same roving
tab-list stop, so pressing Tab exits the list instead of visiting every tab.

An active panel is a Tab stop by default. This lets keyboard and assistive-technology users enter a
panel whose first meaningful content is static text. If a panel begins with an appropriate
focusable control, pass a caller root with `.tab_index(-1)` so normal Tab traversal moves directly
to that child while the panel remains programmatically focusable.

## Mounting and accessibility

Inactive panels are unmounted by default. `.keep_mounted(true)` returns them as `display: none`,
which preserves caller state without contributing layout, paint, input, focus, accessibility, or
runtime work. `panel_part` returns `Option<Element>`, so pass it to `.children(...)` for either
policy. A per-tab `.keep_mounted(...)` override is also available.

The list projects the TabList role and explicit orientation. Each enabled or disabled tab projects
the Tab role, exact selected true/false state, and a relationship to its mounted panel. Each mounted
panel projects the TabPanel role and is labelled by its tab. Disabled tabs expose no native click
action and cannot receive pointer or keyboard activation. Give each visible list an application
label with `.accessibility_label(...)` or `.accessibility_labelled_by(...)`.

## Resource contract

`Tabs`, `Tab`, and `TabsState` contain only IDs, booleans, and one optional active ID. They retain no
item registry, task, timer, observer, animation, GPU resource, or idle scheduler source. The
mounted tree is scanned only when rebuilding Tab order or handling an explicit navigation key.
State changes rebuild only when the caller's listener requests invalidation; a settled tab set
returns to zero extra frames.

Run the caller-styled gallery with:

```console
cargo run --release --example tabs
```

The gallery is one ordinary in-window view—no popover or native child surface. It demonstrates
horizontal manual activation, vertical automatic activation, disabled-item skipping, looping,
retained and unmounted panels, application-owned indicators, and wrapped content.
