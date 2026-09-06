export const en = {
  meta: {
    title: 'QuickGUI — Build native desktop apps. Skip the webview.',
    description:
      'QuickGUI is a GPU-accelerated GUI framework for building native desktop apps in Rust or TypeScript. Familiar Flexbox and Grid layout, real text, built-in accessibility — and zero CPU while your app is idle.',
  },
  common: {
    skipToContent: 'Skip to content',
    getStarted: 'Get started',
    copy: 'Copy “{{text}}”',
    copied: 'Copied',
    language: 'Language',
  },
  nav: {
    features: 'Features',
    code: 'Code',
    quickstart: 'Quickstart',
    docs: 'Docs',
  },
  hero: {
    badge: 'v{{version}} on crates.io',
    titleLine1: 'Build native desktop apps.',
    titleLine2: 'Skip the webview.',
    sub: 'A GPU-accelerated GUI framework for native desktop apps, in Rust or TypeScript. Familiar layout, real text, built-in accessibility — and zero CPU while your app is idle.',
  },
  features: {
    title: 'Everything a desktop app needs.',
    items: {
      idle: {
        title: 'Zero CPU when idle',
        body: 'If nothing changes on screen, nothing runs. Idle windows draw no frames and stay off the battery.',
      },
      fast: {
        title: 'Fast by default',
        body: 'Rendering happens on the GPU, and only the part of the window that changed is redrawn.',
      },
      layout: {
        title: 'Layout you already know',
        body: 'Flexbox and CSS Grid, with Tailwind-style shorthands. If you can lay out a web page, you can lay out a window.',
      },
      components: {
        title: 'Components included',
        body: 'A large collection of accessible, unstyled components — menus, dialogs, popovers, selects, comboboxes, tabs, tables, trees — ready to make yours.',
      },
      text: {
        title: 'Text that just works',
        body: 'Selection, editing, undo, input methods, emoji, and right-to-left scripts behave like they do in every native app.',
      },
      a11y: {
        title: 'Accessible by default',
        body: 'Screen readers see real buttons, lists, and text — no extra code required.',
      },
      lists: {
        title: 'Smooth at any size',
        body: 'Tables and lists with a million rows scroll without dropping frames.',
      },
      native: {
        title: 'Feels native, because it is',
        body: 'Real windows with native menus, dialogs, tray icons, and notifications — not a browser in a costume.',
      },
      cli: {
        title: 'One CLI, dev to release',
        body: 'quickgui dev runs your app and reloads as you edit; quickgui build hands you a signed, installable release.',
      },
    },
  },
  code: {
    title: 'Write it in Rust or TypeScript.',
    lead: 'Use whichever you like — both produce the same native app, with no webview.',
  },
  swiftUi: {
    title: 'Use native SwiftUI in QuickGUI.',
    lead: 'Embed real SwiftUI controls directly in your QuickGUI UI app on macOS.',
  },
  quickstart: {
    title: 'Start in a minute.',
    rust: {
      addCrate: 'Add the crate',
      write: 'Write a view',
      writeBody: 'The counter above is a complete main.rs — copy it in.',
      run: 'Run it',
    },
    ui: {
      create: 'Create an app',
      ship: 'Ship it',
      note: 'You get a real, signed, self-contained app — installer included.',
    },
  },
  platforms: {
    available: 'available now',
    soon: 'in progress',
  },
  cta: {
    title: 'Build something native.',
    body: 'Write a view, ship a real app — and let idle windows actually idle.',
    docs: 'Read the docs',
    star: 'Star on GitHub',
  },
  footer: {
    license: 'MIT or Apache-2.0',
  },
}
