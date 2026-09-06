# Herdr GUI

A real, standalone GUI interpretation of [Herdr](https://herdr.dev/) built with QuickGUI and
QuickGUI UI. It does not call or embed the Herdr CLI.

From the repository root:

```sh
bun run build:native
bun run --filter herdr-gui dev
```

The app owns real pseudoterminals and child processes. Its model follows Herdr itself: spaces are
project workspaces, spaces own tabs, and tabs own tiled terminal panes. The sidebar's lower panel is
derived only from agents that the Rust core discovers in live PTY foreground process groups. A
plain terminal never becomes a sidebar agent until an actual supported agent process is running.
Idle, working, and blocked states are derived from the live process and terminal screen.

The New Agent sheet is only a convenient direct launcher. It is not the sidebar's source of truth,
and the app never installs, calls, embeds, or shells out to the Herdr CLI.

The UI follows the system appearance by default, supports explicit light and dark modes, persists
spaces and appearance, and has captured-pointer resizing for the sidebar and its spaces/agents
split. Terminal parsing, keyboard encoding, adaptive default colors, resize, scrollback, process
detection, and cleanup live in QuickGUI's Rust core with `libghostty-vt`. PTYs identify themselves
with `TERM_PROGRAM=ghostty`.

The terminal uses the executable-embedded Regular, Bold, Italic, and Bold Italic faces of
`JetBrainsMono Nerd Font Mono` from Nerd Fonts 3.5.1 (SIL OFL 1.1; the license is packaged with
the app). Its light and dark foreground, background, cursor, and 16 ANSI colors are the exact
GitHub Light Default and GitHub Dark Default terminal values from GitHub's VS Code theme 6.3.5.
The bundled font files have these SHA-256 checksums:

- Regular: `f2a5ea6cfab397445ffab00c0370927b66d61e560a05db5db271b42006381c1a`
- Bold: `bfcf9a917276ffc058867d87cbc8a5b2f1ab0f4b710e9170dc02763ccb80bd4b`
- Italic: `31efd6ead98746f5b0afa1ee6dba60267ad48db36428360bee327bec10621f97`
- Bold Italic: `9dba502e00e35209f6ed2a151c7376c051657b067cdebbc6e52d06cb9002cf31`

Closing an agent tab terminates that process. Unlike Herdr's background server, this example keeps
sessions alive only while the GUI process is running.
