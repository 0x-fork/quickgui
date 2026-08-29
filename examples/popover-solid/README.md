# QuickGUI Solid popovers

This example compares the two controlled JSX popover surfaces available to a Solid application:

- `SystemPopover` portals its children through a separate Solid renderer into a parent-owned native
  child window. It may cross the owner edge and fits against the display work area.
- `Popover` stays inside the current window's retained overlay plane. Core placement
  flips and shifts it within the content viewport and handles Escape or outside-press dismissal.

Both expose the same compound `Root`, `Trigger`, and `Content` API. The root owns controlled or
uncontrolled open state, the trigger registers its native anchor internally, and the content part
owns size and placement.

```console
cd examples/popover-solid
bun run dev
```
