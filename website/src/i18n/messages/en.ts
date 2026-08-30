export const en = {
  meta: {
    title: 'QuickGUI — Desktop UI that only renders what changed.',
    description:
      'QuickGUI is a damage-driven, GPU-accelerated GUI framework for Rust desktop apps: a GPUI-style fluent API, Flexbox and CSS Grid, retained Unicode text, native accessibility, and bounded virtual scrolling. Clean windows sleep, so idle apps render zero frames.',
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
    architecture: 'Architecture',
    quickstart: 'Quickstart',
    docs: 'Docs',
  },
  hero: {
    badge: 'v{{version}} / now on crates.io',
    title: 'Desktop UI that only renders what changed.',
    sub: 'QuickGUI is a damage-driven, GPU-accelerated GUI framework for Rust. GPUI-style fluent views, Flexbox and CSS Grid, retained text, native accessibility — and windows that sleep at zero idle frames.',
    caption: 'a live model of the damage-driven loop — click Increment',
  },
  stats: {
    idle: 'idle frames when clean',
    scroll: 'scrolling 100,000 rows',
    cpu: 'p95 frame CPU',
    tests: 'tests in the core suite',
  },
  features: {
    eyebrow: 'Why QuickGUI',
    title: 'A framework that respects the machine.',
    lead: 'QuickGUI retains layout, shaped text, scenes, and GPU caches between frames — then repaints only the damage.',
    alsoInTheBox: 'Also in the box',
    items: {
      sleeps: {
        title: 'Sleeps when idle',
        body: 'Clean windows wait in ControlFlow::Wait and render no idle frames. Hover, scroll, and drag repaint retained geometry — no view rebuild, no relayout.',
      },
      gpu: {
        title: 'Damage-driven GPU rendering',
        body: 'WGPU pipelines for instanced shapes, cached text, paths, images, SVGs, shadows, and custom WGSL. Compatible windows share one device and queue.',
      },
      layout: {
        title: 'Layout you already know',
        body: 'Taffy Flexbox and CSS Grid with Tailwind-style helpers, plus parent-size container queries that redeclare only when their assigned box changes.',
      },
      text: {
        title: 'Text as a first-class citizen',
        body: 'Retained Unicode shaping via Cosmic Text: OpenType features, ordered fallbacks, BiDi, ellipsis and line clamp, IME, selection, editing, and undo.',
      },
      a11y: {
        title: 'Accessible by construction',
        body: 'AccessKit trees expose real roles, relationships, and actions. VoiceOver sees actual controls — not a picture of controls.',
      },
      virtual: {
        title: 'Bounded virtualization',
        body: 'Uniform and measured variable-height lists mount only visible rows and keep logical anchors. Million-row tables and trees stay bounded.',
      },
    },
    chips: {
      menus: 'native menus',
      dialogs: 'dialogs & sheets',
      popovers: 'NSPanel popovers',
      select: 'select / autocomplete / combobox',
      tabs: 'tabs',
      collections: 'virtual tables & trees',
      dnd: 'drag & drop',
      clipboard: 'clipboard',
      notifications: 'notifications',
      tray: 'tray icons',
      motion: 'springs & transitions',
      shaders: 'custom shaders',
      tests: 'deterministic tests',
      inspector: 'retained-tree inspector',
    },
  },
  code: {
    eyebrow: 'View API',
    title: 'Views are ordinary code.',
    lead: 'The same counter, twice: GPUI-style fluent Rust, or Solid 2 JSX running natively on Bun — no webview, no virtual DOM.',
    points: {
      builders: {
        title: 'Ordinary code, fluent builders',
        body: 'JSX-like composition with Tailwind-vocabulary helpers: .flex_col().items_center().gap_3(). No macros required.',
      },
      listeners: {
        title: 'Typed, view-local listeners',
        body: 'cx.listener binds events to your view state. Mutate, invalidate, done — the framework schedules exactly one frame.',
      },
      paint: {
        title: 'Paint-only interaction states',
        body: 'hover, active, focus, and drag variants repaint retained geometry. Add .transition() to interpolate without relayout.',
      },
    },
    viewApiDocs: 'View API docs',
    solidDocs: 'Solid renderer docs',
  },
  architecture: {
    eyebrow: 'Architecture',
    title: 'A frame only happens when something changes.',
    lead: 'Each stage is retained and reused until its inputs actually change. If nothing changed, no frame is produced — clean windows park in ControlFlow::Wait.',
    pipeline: {
      input: {
        title: 'Input events',
        body: 'Winit + AccessKit, coalesced at the event-loop boundary.',
      },
      tree: {
        title: 'Retained element tree',
        body: 'Rebuilt only for app changes; keyed listeners and state survive.',
      },
      layout: {
        title: 'Taffy layout',
        body: 'Flexbox, Grid, and container queries — rerun only on layout changes.',
      },
      scene: {
        title: 'Reusable scene',
        body: 'Ordered planes and z-layers; hover and scroll reuse it as-is.',
      },
      submit: {
        title: 'WGPU submit',
        body: 'Instanced shapes, cached glyphs, retained paths — one bounded pass.',
      },
    },
    docsLink: 'Read the architecture docs',
    demoCaption: 'same idea, in your browser — only the visible rows exist',
    gatesTitle: 'The numbers are gated, not vibes.',
    gatesBody:
      'Self-terminating macOS gates scroll a live 100,000-row list against a real WindowServer and enforce frame-time, CPU, memory, cache, and idle budgets.',
    gates: {
      scroll: 'sustained bidirectional scroll',
      frameCpu: 'p95 frame CPU',
      processCpu: 'whole-process CPU',
      rss: 'peak RSS',
      idle: 'extra idle frames',
    },
    gatesLink: 'Run the gates yourself',
  },
  statement: {
    text: 'Most desktop pixels are rectangles, glyphs, icons, and images. QuickGUI dedicates its pipelines to exactly that workload — and lets everything else sleep.',
    link: 'Why this renderer',
  },
  quickstart: {
    eyebrow: 'Quickstart',
    title: 'From zero to a native window.',
    lead: 'Use the Rust crate directly, or write it in Solid and ship with the QuickGUI CLI.',
    rust: {
      addCrate: 'Add the crate',
      write: 'Write a view',
      writeBody:
        'The <lnk>counter above</lnk> is a complete <c>main.rs</c> — fluent views, typed listeners, no macros.',
      run: 'Run it',
      footnote:
        'Rust 2024 edition · macOS 0.1 accepted · docs on <lnk>docs.rs</lnk>',
    },
    solid: {
      scaffold: 'Scaffold and run a dev app',
      ship: 'Ship a signed build',
      body: '<c>quickgui dev</c> runs a real signed .app that hot-restarts on edits; <c>quickgui build</c> produces a self-contained .app and versioned DMG with notarization built in.',
      footnote: 'Bun 1.3+ · no webview, no virtual DOM · <lnk>CLI docs</lnk>',
    },
  },
  platforms: {
    eyebrow: 'Platforms',
    title: 'Honest about where it runs.',
    lead: 'Milestones are evidence-gated, not date-gated. Compiling is never promoted to native proof.',
    status: {
      accepted: 'accepted',
      compiles: 'compiles',
    },
    items: {
      macos:
        'The 0.1 platform: native windows, menus, dialogs, NSPanel popovers, IME, VoiceOver projection, and live performance gates.',
      windows:
        'Compiles through Winit + WGPU today. Native runtime, visual, and accessibility acceptance is the 0.3 milestone.',
      linux:
        'Compiles through Winit + WGPU today. Native runtime, visual, and accessibility acceptance is the 0.3 milestone.',
    },
    roadmapTitle: 'Roadmap',
    shipped: 'shipped',
    roadmap: {
      m1: {
        label: 'macOS-first foundation',
        detail: 'Shipped to crates.io on 2026-08-27.',
      },
      m2: {
        label: 'Unstyled component contract',
        detail:
          'Popover menus, select, combobox, dialogs, tables, trees — live-accepted on macOS.',
      },
      m3: {
        label: 'Windows & Linux parity',
        detail:
          'Native visual, IME, accessibility, and performance gates on both platforms.',
      },
      m4: {
        label: 'Stable cross-platform contract',
        detail:
          'Documented compatibility policy and green resource budgets everywhere.',
      },
    },
    statusLink: 'Full status & roadmap',
  },
  cta: {
    title: 'Build something native.',
    body: 'Add the crate, write a view, ship a signed app — and let idle windows actually idle.',
    docs: 'Read the docs',
    star: 'Star on GitHub',
  },
  footer: {
    description:
      'A damage-driven, GPU-accelerated GUI framework for Rust desktop applications.',
    project: 'Project',
    packages: 'Packages',
    community: 'Community',
    links: {
      docs: 'Documentation',
      viewApi: 'View API',
      architecture: 'Architecture',
      status: 'Status & roadmap',
      changelog: 'Changelog',
      crate: 'quickgui on crates.io',
      docsRs: 'API docs on docs.rs',
      solid: 'Solid 2 renderer',
      cli: 'CLI & packaging',
      github: 'GitHub',
      issues: 'Issues',
      examples: 'Examples',
      license: 'License',
    },
    license: 'MIT or Apache-2.0, at your option',
  },
}
