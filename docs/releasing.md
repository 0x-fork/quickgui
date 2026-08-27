# Releasing QuickGUI 0.1

[Documentation index](README.md)

QuickGUI 0.1 stays on the stable Winit 0.30 type universe used by `accesskit_winit` 0.33.2.
The versioned `quickgui-winit` support package carries macOS panel allocation, touch delivery, and
the pre-buffer AppKit mouse click count required by the runtime, while
`quickgui-accesskit-winit` changes only the adapter's Winit dependency.
`quickgui-cosmic-text` adds ordered per-style fallback families to shaping and owned cache keys;
`quickgui-glyphon` adds a per-text-area opacity multiplier after rich-run color resolution so
opacity transitions remain paint-only rather than invalidating shaping and depends on that exact
Cosmic Text support version. Winit 0.31 beta exposes
native panels upstream, but also changes the window and event-loop interfaces; migrating to that
beta is not part of the 0.1 release boundary.

The main crate's minimum supported Rust version is 1.89, matching Cosmic Text 0.19 as selected by
Glyphon 0.12. CI compiles every target and feature with that exact toolchain on Linux in addition
to the stable macOS quality job.

## Automated release gate

Start from a clean checkout of the intended tag and run:

```console
cargo fmt --all -- --check
cargo test --all-targets --all-features --locked
cargo check --all-targets --all-features --locked
cargo +1.89.0 check --all-targets --all-features --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features --locked
bash -n scripts/*.sh
git diff --check
QUICKGUI_PACKAGE_TOOLCHAIN=1.89.0 scripts/package-release-gate.sh
```

The package gate builds all five `.crate` archives, extracts the exact normalized contents, and
compiles `tests/downstream_smoke` with only those extracted packages patched into the registry
graph. This catches missing files, accidental path-only dependencies, mismatched renamed-crate
types, missing license/notice files, duplicated vendor sources, and a public API that cannot be
consumed outside this repository. It rejects a dirty source tree by default;
`QUICKGUI_PACKAGE_ALLOW_DIRTY=1` exists only for development verification.
CI sets `QUICKGUI_PACKAGE_TOOLCHAIN=1.89.0`, making both package creation and the fresh downstream
resolution use the declared MSRV rather than the runner's newer default compiler.

The GitHub `CI` workflow runs this complete non-interactive gate on clean commits. Pushes, tags, and
manual dispatches upload one immutable `quickgui-<version>-crates-<commit>` artifact containing all
five verified `.crate` archives, `SHA256SUMS`, this release guide, and the changelog. Pull requests
verify the same packages but do not retain release artifacts. A tag build fails unless the tag is
exactly `v<package-version>`; for 0.1.0 that is `v0.1.0`. Ordinary CI never publishes a crate.

## macOS acceptance evidence

The live probes are separate because they create native windows and measure a real WindowServer:

```console
scripts/macos-performance-gate.sh
scripts/macos-acceptance-gate.sh
scripts/macos-display-acceptance-gate.sh
```

Run a focused probe after changing the subsystem it covers. Do not repeat the complete composition
and 128-cycle popup soak merely because documentation, packaging, examples, or unrelated component
code changed. A release decision may combine the last passing full composition probe with newer
focused performance or display evidence, provided the intervening change and its focused coverage
are recorded in [the status ledger](status.md).

## Publication order

Publishing changes external registry state and is never part of an ordinary build or test. After
reviewing the archive file lists and checksums emitted by the package gate, publish in dependency
order:

```console
cargo publish --manifest-path vendor/winit/Cargo.toml
cargo publish --manifest-path vendor/accesskit_winit/Cargo.toml
cargo publish --manifest-path vendor/cosmic_text/Cargo.toml
cargo publish --manifest-path vendor/glyphon/Cargo.toml
cargo publish --locked
```

Wait for each support version to become resolvable from crates.io before publishing its dependent.
The versions are intentionally exact: `quickgui-winit = 0.30.13-quickgui.1`, then
`quickgui-accesskit-winit = 0.33.2-quickgui.1`; independently publish
`quickgui-cosmic-text = 0.19.0-quickgui.1`, then `quickgui-glyphon = 0.12.0-quickgui.1`, before
`quickgui = 0.1.0`. Never rerun a successful publish;
registry releases are immutable, so any correction requires a new version.

QuickGUI 0.1 was the first release of these package names and was published manually on 2026-08-27.
crates.io Trusted Publishing can now replace long-lived API tokens after explicit publisher
configuration; it is intentionally not guessed or enabled by this workflow.

Finally, create a fresh crate outside this repository, add `quickgui = "=0.1.0"` without any
`[patch]` or path dependency, and run `cargo check`. Run all three live macOS gates once more from the
tagged source if the published archives differ from the previously recorded checksums.

The 2026-08-27 publication completed this check with Rust 1.89 using only the indexed crates.io
packages; all five exact versions resolved and the fresh downstream crate compiled successfully.
