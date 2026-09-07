# @quickgui/native-updater

Optional automatic updates for QuickGUI Go applications. Import `github.com/egoist/quickgui/go/updater`; the CLI resolves this package at the same release version as the Go SDK. It is not a dependency of the default native core.

macOS uses the bundled Sparkle 2.9.4 framework. Windows and Linux implement its signed appcast contract with a native installer helper. All app/extension calls use the existing in-process purego bridge. The helper runs separately only to install after the app releases its executable and libraries.

Build from the repository with `bun packages/native/build.ts --extension updater`. Current npm release artifacts cover macOS arm64 and x64; other platforms build from source on their target hosts. Configure, sign, and publish updates using the [updater guide](https://github.com/egoist/quickgui/blob/master/docs/updater.md).
