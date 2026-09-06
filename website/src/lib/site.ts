export const site = {
  name: 'QuickGUI',
  tagline: 'Build native desktop apps. Skip the webview.',
  description:
    'QuickGUI is a GPU-accelerated GUI framework for building native desktop apps in Rust or TypeScript. Familiar Flexbox and Grid layout, real text, built-in accessibility — and zero CPU while your app is idle.',
  repo: 'egoist/quickgui',
  /** Fallback when the crates.io lookup is unavailable. */
  version: '0.1.0',
  links: {
    github: 'https://github.com/egoist/quickgui',
    docs: '/docs',
    viewApi: 'https://github.com/egoist/quickgui/blob/main/docs/view-api.md',
    ui: 'https://github.com/egoist/quickgui/blob/main/docs/ui.md',
    cli: 'https://github.com/egoist/quickgui/blob/main/docs/cli.md',
    status: 'https://github.com/egoist/quickgui/blob/main/docs/status.md',
    architecture:
      'https://github.com/egoist/quickgui/blob/main/docs/architecture/README.md',
    examples: 'https://github.com/egoist/quickgui/tree/main/examples',
    changelog: 'https://github.com/egoist/quickgui/blob/main/CHANGELOG.md',
    crate: 'https://crates.io/crates/quickgui',
    docsRs: 'https://docs.rs/quickgui',
    license: 'https://github.com/egoist/quickgui/blob/main/LICENSE-MIT',
  },
} as const
