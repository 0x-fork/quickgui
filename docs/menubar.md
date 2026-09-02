# In-window menubar

[Documentation index](README.md) · [Unstyled component roadmap](component-roadmap.md)

QuickGUI's native [`Menu`](desktop-integrations.md) remains the application menu on macOS and
Windows. `MenubarState` is the separate in-window component for products that draw their own
command bar inside the window — a hidden-inset title bar, a cross-platform editor chrome, or an
embedded tool surface.

The bar owns roving focus, which menu is open, hover switching, and menubar accessibility. Each
surface is an ordinary [`PopoverMenu`](popovers.md), so items, groups, separators, checkbox and
radio rows, submenus, typeahead, and typed command dispatch are unchanged. The application owns
every color, size, separator, row, and the placement of each open surface.

The behavior follows the WAI-ARIA [Menubar](https://www.w3.org/WAI/ARIA/apg/patterns/menubar/)
pattern.

Run the caller-styled gallery with:

```console
cargo run --release --example menubar
```

## Composition

`MenubarState::new(menu_count)` retains the declared menu count, the menu that owns the bar's single
Tab stop, and the menu that is open. It is a plain copyable value.

```rust
use quickgui::{Menubar, MenubarState, div, text};

struct Shell {
    menubar: MenubarState, // MenubarState::new(3)
    file: PopoverMenu,
}

let bar = Menubar::new("menubar");
let mut row = bar
    .root_part(self.menubar, div().flex_row())
    .accessibility_label("Application menus");
for index in 0..self.menubar.menu_count() {
    let item = bar.item(self.menubar, index).expect("a declared menu");
    let element = item.item_part(div().px(10.0).child(text(TITLES[index])));
    row = row.child(item.key_part(cx, element, |view: &mut Shell| &mut view.menubar));
}
```

`root_part` and `item_part` are pure decorators; they add identity, roles, relationships, and
interaction contracts and never add layout or paint. `key_part` attaches the typed keyboard actions,
click opening, and hover switching to one trigger. `Menubar::item` returns `None` past
`MenubarState::menu_count`, so a caller-driven loop stays bounded by the state instead of by its own
arithmetic.

Mount the open surface yourself, anchored to `MenubarItem::item_id`, and close the bar from the
menu's `dismiss` callback:

```rust
let surface = self.menubar.open_menu().map(|index| {
    self.menu_for(index).element(
        cx,
        "menu-surface",
        Shell::menu_for_mut,
        div().absolute().top(30.0),
        render_item,
        |view: &mut Shell, cx| {
            view.menubar.close();
            cx.invalidate();
        },
    )
});
```

## Keyboard and pointer

Install `menubar_key_bindings()` once. Left and Right move between menus and wrap at both ends, Home
and End jump to the first and last menu, Down, Return, and Space open the focused menu, and Escape
closes the open menu without leaving the bar. Arrow keys keep working while a menu is open: they
switch menus rather than closing, which is the desktop menubar convention.

Exactly one trigger carries the bar's Tab stop; the rest are reached with the arrow keys. Clicking a
title opens it, clicking it again closes it, and hovering another title switches menus **only while
one is already open** — hovering a closed bar does nothing, exactly like macOS and Windows.

## Accessibility

The root projects the menubar role with a horizontal orientation, the declared menu count, and an
active-descendant relationship to the open menu, or to the focused one while the bar is closed.
Every trigger is a menu item that declares it controls a menu popup, exposes its expanded state and
zero-based position in the set, and reports `selected` while its menu is open.

## Bounds

| Constant | Value | Meaning |
| --- | --- | --- |
| `MAX_MENUBAR_MENUS` | 64 | Top-level menus one menubar manages. |

A larger declared count is clamped rather than rejected, so a dynamic application menu can never
grow the retained state without bound. `set_menu_count` clamps focus and closes a menu that no
longer exists.

`MenubarState` retains three integers and a flag. It holds no item registry, allocation, task,
timer, observer, animation, GPU resource, or idle scheduler source, and it cannot produce a
deadline. A settled bar renders zero extra frames.
