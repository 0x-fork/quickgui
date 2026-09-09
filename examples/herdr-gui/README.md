# Herdr GUI

A standalone QuickGUI application inspired by Herdr. The UI and application state are Go; the native core owns the retained PTYs, terminal rendering, input, and process detection. It does not embed or call the Herdr CLI.

- **Spaces, tabs, and panes:** each project directory has its own tabs and remembers the selected tab. Split right or down to tile terminals. Switching tabs, spaces, or appearance preserves each PTY and its scrollback.
- **Live Agents sidebar:** Codex, Claude Code, OpenCode, and other supported agents appear when the native terminal detects their foreground process. Working, idle, and needs-input indicators come from that process and its terminal output. A shell is not counted as an agent merely because of how its tab was created.
- **New Agent sheet:** discovers installed Codex, Claude, and OpenCode executables through the interactive login-shell environment and common installation paths. Choose a launcher and an optional initial instruction. Nothing is installed automatically.
- **Window controls:** drag the sidebar edge and the Spaces/Agents divider. The View menu offers system, light, and dark appearance. Spaces, the active space, both divider positions, and appearance persist in `herdr-gui-state.json` in the application's data directory. The original example's state format is preserved; the reduced Go example's state is imported when no original state exists.
- **Terminal appearance:** bundled JetBrainsMono Nerd Font Mono regular, bold, italic, and bold italic faces; GitHub Light Default and Dark Default ANSI palettes; a matching cursor and extended edge backgrounds. Fonts are loaded relative to the application's resources, independently of its launch directory.

`Cmd+T` opens a tab, `Cmd+N` opens the agent sheet, `Cmd+O` adds a space, and `Cmd+D` / `Cmd+Shift+D` split right / down. `Cmd+W` closes the focused split, then the tab when other tabs remain, then the window. Native copy, paste, hide, minimize, zoom, and full-screen actions remain available. Closing a pane terminates its PTY; running sessions are not restored after quitting.

From the repository root:

```console
bun install
bun run build:native # after native core changes
bun packages/cli/src/cli.ts dev --project examples/herdr-gui
```

Application edits rebuild only Go. The shared library is loaded in process with purego; no CGO or frontend IPC is used.

```console
bun packages/cli/src/cli.ts test --project examples/herdr-gui
bun packages/cli/src/cli.ts fmt --project examples/herdr-gui
```

[main.go](main.go) handles startup, menus, and window events; [model.go](model.go) manages spaces, tabs, and panes. [workspace.go](workspace.go), [sidebar.go](sidebar.go), and [agent_sheet.go](agent_sheet.go) declare the UI. [model_test.go](model_test.go) covers selection, closing, native-node retention, agent status, launch arguments, discovery, and persistence.

The four embedded font faces are distributed under the [SIL Open Font License](assets/JetBrainsMonoNerdFont-OFL.txt). See the [Go guide](../../docs/go.md) for framework usage.
