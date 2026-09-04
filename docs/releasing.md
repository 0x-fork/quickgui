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

The main crate's minimum supported Rust version is 1.90, matching `libghostty-vt` 0.2.1 as selected
by the optional terminal feature. CI compiles every target and feature with that exact toolchain on
Linux in addition to the stable macOS quality job.

## Automated release gate

Start from a clean checkout of the intended tag and run:

```console
cargo fmt --all -- --check
cargo test --all-targets --all-features --locked
cargo check --all-targets --all-features --locked
cargo +1.90.0 check --all-targets --all-features --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features --locked
bash -n scripts/*.sh
git diff --check
QUICKGUI_PACKAGE_TOOLCHAIN=1.90.0 scripts/package-release-gate.sh
```

The package gate builds all six `.crate` archives, extracts the exact normalized contents, and
compiles `tests/downstream_smoke` with only those extracted packages patched into the registry
graph. This catches missing files, accidental path-only dependencies, mismatched renamed-crate
types, missing license/notice files, duplicated vendor sources, and a public API that cannot be
consumed outside this repository. Each archive is also restricted to a per-crate top-level
allowlist, so examples, benches, tests, repository docs, scripts, and workflow files cannot leak
into future crates.io releases. It rejects a dirty source tree by default;
`QUICKGUI_PACKAGE_ALLOW_DIRTY=1` exists only for development verification.
CI sets `QUICKGUI_PACKAGE_TOOLCHAIN=1.90.0`, making both package creation and the fresh downstream
resolution use the declared MSRV rather than the runner's newer default compiler.

The GitHub `CI` workflow runs this complete non-interactive gate on clean commits. Every non-PR
invocation uploads one immutable `quickgui-<version>-crates-<commit>` artifact containing all six
verified `.crate` archives, `SHA256SUMS`, this release guide, and the changelog. Pull requests verify
the same packages but do not retain release artifacts. The CI workflow never publishes.

A pushed `v*` tag starts the separate `Release` workflow, which invokes the reusable CI workflow
and waits for its macOS, Windows, and Linux gates before publishing. The publish job rejects a tag
that is not exactly `v<root-package-version>` or lacks a dated changelog section. The root
`package.json` version is the source of truth; the release gate requires the `quickgui` and
`quickgui-system` crates, native binding crate, and all three npm packages to match it. The job
builds both macOS native architectures, runs the JavaScript tests and typecheck, verifies the npm
tarballs, publishes both registries in dependency order, installs the public packages in fresh
Rust and Bun consumers, and only then creates the GitHub Release.

## macOS acceptance evidence

The live probes are separate because they create native windows and measure a real WindowServer:

```console
scripts/macos-performance-gate.sh
scripts/macos-acceptance-gate.sh
scripts/macos-display-acceptance-gate.sh
```

Run a focused probe after changing the subsystem it covers. Do not repeat the complete composition
and 128-cycle popover soak merely because documentation, packaging, examples, or unrelated component
code changed. A release decision may combine the last passing full composition probe with newer
focused performance or display evidence, provided the intervening change and its focused coverage
are recorded in [the status ledger](status.md).

## Registry authentication setup

The private GitHub repository cannot use crates.io trusted publishing. Create a crates.io API token
that can publish these six crates and store it as the `CARGO_REGISTRY_TOKEN` GitHub Actions secret:

- `quickgui-winit`
- `quickgui-accesskit-winit`
- `quickgui-cosmic-text`
- `quickgui-glyphon`
- `quickgui-system`
- `quickgui`

For each npm package (`@quickgui/native`, `@quickgui/solid`, and `@quickgui/cli`), add an
[npm Trusted Publisher](https://docs.npmjs.com/trusted-publishers/) with GitHub owner `egoist`,
repository `quickgui`, workflow filename `release.yml`, and no environment. Allow `npm publish`.
npm requires Node 22.14 or newer and npm 11.5.1 or newer for OIDC; the workflow uses Node 24 and
verifies the npm CLI before publication. No `NPM_TOKEN` secret is required.

Creating the workflow does not create the npm registry-side trust records. A missing or misspelled
record makes npm authentication fail before publication.

## Tag-driven publication

Prepare the release commit by setting the version in the root `package.json` and mirroring it into
the `quickgui`, `quickgui-system`, native binding, and three npm package manifests. Update the CLI
version and generated-project dependency versions to match. The four vendored compatibility forks
retain upstream-derived versions; bump a fork and its exact dependency only when that fork changes.
Move the shipped changes out of `Unreleased` into a dated `## <version> - YYYY-MM-DD` section. Then
create and push an annotated `v<version>` tag:

```console
git tag -a v0.1.2 -m "QuickGUI 0.1.2"
git push origin main v0.1.2
```

Pushing the tag starts the release automatically. To start or retry it manually, open the `Release`
workflow, choose **Run workflow**, and enter the existing tag such as `v0.1.2`. Manual dispatch does
not create the tag, update package versions, or change the changelog; prepare and push those first.

Every first-party QuickGUI Cargo and npm package uses this one release version. The metadata gate
reads it from the root `package.json` and fails before packaging if any mirrored version differs.

The workflow publishes crates.io packages in this dependency order:

1. `quickgui-winit`
2. `quickgui-accesskit-winit`
3. `quickgui-cosmic-text`
4. `quickgui-glyphon`
5. `quickgui-system`
6. `quickgui`

It then publishes npm packages in the order `@quickgui/native`, `@quickgui/solid`, and
`@quickgui/cli`. Each dependent waits until the previous package is anonymously resolvable from
its public registry. A rerun skips an existing, non-yanked crate version and skips an existing npm
version only when its registry integrity matches the locally verified tarball. This permits safe
recovery from a partial registry release without attempting to overwrite immutable versions.

The npm tarballs and their SHA-256 checksums are retained as a workflow artifact and attached to the
GitHub Release. npm trusted publishing works for the private repository, but does not generate
provenance for it.

## Manual recovery

If automation is unavailable, use the same dependency order from a clean, fully verified tag:

```console
cargo publish --manifest-path vendor/winit/Cargo.toml
cargo publish --manifest-path vendor/accesskit_winit/Cargo.toml
cargo publish --manifest-path vendor/cosmic_text/Cargo.toml
cargo publish --manifest-path vendor/glyphon/Cargo.toml
cargo publish --manifest-path crates/quickgui-system/Cargo.toml --locked
cargo publish --locked
```

Pack npm packages with `bun pm pack`, which resolves `workspace:*` dependencies to their exact
workspace versions, and publish the resulting tarballs with npm 11.5.1 or newer. Do not publish the
workspace directories with npm directly. Wait for each package to propagate before its dependent.
Never rerun a successful manual publish; first inspect the public registry and continue after the
last completed package.

Finally, run `scripts/release-registry-smoke.sh <version>` to compile a fresh Rust 1.90 consumer
without patches, install all three packages at that same version in a fresh Bun project, import the
native and Solid runtimes, and execute the installed CLI. Run the live macOS gates once more from
the tagged source if the published artifacts differ from the previously recorded candidates.
