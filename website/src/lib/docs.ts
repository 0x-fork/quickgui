import type { Locale } from '../i18n'
import { MOONBIT_DOCS_PAGES } from './moonbit-docs'

export const DOCS_FRONTENDS = ['go', 'moonbit'] as const
export type DocsFrontend = (typeof DOCS_FRONTENDS)[number]

export type GoDocsSlug =
  | 'getting-started'
  | 'project-structure'
  | 'updater'
  | 'extensions'
  | 'ui'
  | 'styling'
  | 'animations'
  | 'components'
  | 'forms-and-input'
  | 'overlays-and-dialogs'
  | 'swift-ui'
  | 'swift-ui-hosting'

export type DocsSlug = GoDocsSlug | 'native-services'

export interface DocsOutlineItem {
  id: string
  title: string
  level?: 2 | 3
}

export interface DocsPageTranslation {
  title: string
  description: string
  outline: readonly DocsOutlineItem[]
  searchTerms: readonly string[]
}

export interface DocsPageMeta extends DocsPageTranslation {
  frontend: DocsFrontend
  slug: DocsSlug
  translations?: Partial<Record<Locale, Omit<DocsPageTranslation, 'searchTerms'>>>
}

export const GO_DOCS_PAGES: readonly DocsPageMeta[] = [
  {
    frontend: 'go',
    slug: 'extensions',
    title: 'Authoring Extensions',
    description: 'Share Go components and build optional native providers for QuickGUI.',
    outline: [
      { id: 'choose-an-extension-type', title: 'Choose an extension type' },
      { id: 'create-an-extension-project', title: 'Create an extension project' },
      { id: 'share-a-go-component', title: 'Share a Go component' },
      { id: 'lay-out-a-native-extension', title: 'Lay out a native extension' },
      { id: 'declare-the-go-dependency', title: 'Declare the Go dependency' },
      { id: 'implement-the-native-contract', title: 'Implement the native contract' },
      { id: 'automatic-registration', title: 'Automatic registration' },
      { id: 'build-and-package-artifacts', title: 'Build and package artifacts' },
      { id: 'test-and-distribute', title: 'Test and distribute' },
    ],
    searchTerms: [
      'extension',
      'authoring',
      'init-extension',
      'zig',
      'rust',
      'plugin',
      'native provider',
      'third-party',
      'purego',
      'manifest',
      'ABI',
      'ServiceApi',
      'RequireExtension',
      'InvokeExtension',
      'OpenExtension',
      'npm',
    ],
  },
  {
    frontend: 'go',
    slug: 'updater',
    title: 'Auto Updater',
    description: 'Add optional Sparkle-compatible updates to a Go application.',
    outline: [
      { id: 'configuration', title: 'Configuration' },
      { id: 'application-usage', title: 'Application usage' },
      { id: 'lifecycle', title: 'Lifecycle' },
      { id: 'platforms', title: 'Platforms' },
      { id: 'publishing', title: 'Publishing' },
      { id: 'native-artifacts', title: 'Native artifacts' },
    ],
    searchTerms: ['updater', 'sparkle', 'appcast', 'ed25519', 'extension', 'updates'],
  },
  {
    frontend: 'go',
    slug: 'getting-started',
    title: 'Getting Started',
    description: 'Create and run a native QuickGUI app with QuickGUI UI.',
    outline: [
      { id: 'requirements', title: 'Requirements' },
      { id: 'create-a-project', title: 'Create a project' },
      { id: 'your-first-window', title: 'Your first window' },
      { id: 'run-and-build', title: 'Run and build' },
      { id: 'next-steps', title: 'Next steps' },
    ],
    searchTerms: ['install', 'create', 'cli', 'window', 'bun', 'macos'],
  },
  {
    frontend: 'go',
    slug: 'project-structure',
    title: 'Project Structure',
    description: 'Understand the files in a generated QuickGUI UI project.',
    outline: [
      { id: 'generated-files', title: 'Generated files' },
      { id: 'application-entry', title: 'Application entry' },
      { id: 'configuration', title: 'Configuration' },
      { id: 'packages', title: 'Packages' },
    ],
    searchTerms: ['files', 'config', 'entry', 'package', 'quickgui.config'],
  },
  {
    frontend: 'go',
    slug: 'ui',
    title: 'QuickGUI UI Usage',
    description: 'Use QuickGUI UI reactivity to render retained native UI.',
    outline: [
      { id: 'rendering-model', title: 'Rendering model' },
      { id: 'reactive-state', title: 'Reactive state' },
      { id: 'window-lifecycle', title: 'Window lifecycle' },
      { id: 'current-window', title: 'Current window' },
      { id: 'routing', title: 'Routing' },
    ],
    searchTerms: ['go', 'signal', 'component', 'reactivity', 'lifecycle', 'router'],
  },
  {
    frontend: 'go',
    slug: 'styling',
    title: 'Styling & Layout',
    description: 'Lay out and style native nodes with familiar properties.',
    outline: [
      { id: 'the-style-prop', title: 'The style prop' },
      { id: 'merging-styles', title: 'Merging styles' },
      { id: 'flexbox', title: 'Flexbox' },
      { id: 'grid', title: 'Grid' },
      { id: 'text-and-color', title: 'Text and color' },
      { id: 'interaction-states', title: 'Interaction states' },
      { id: 'groups-and-named-group-hover', title: 'Groups and named group hover' },
    ],
    searchTerms: ['style', 'layout', 'flexbox', 'grid', 'color', 'hover'],
  },
  {
    frontend: 'go',
    slug: 'animations',
    title: 'Transitions & Animation',
    description: 'Animate native style changes and images with retained Go components.',
    outline: [
      { id: 'hover-transitions', title: 'Hover transitions' },
      { id: 'state-driven-animation', title: 'State-driven animation' },
      { id: 'timing-and-frame-rate', title: 'Timing and frame rate' },
      { id: 'supported-properties', title: 'Supported properties' },
      { id: 'mounting-and-reduced-motion', title: 'Mounting and reduced motion' },
      { id: 'animated-images', title: 'Animated images' },
    ],
    searchTerms: [
      'transition',
      'animation',
      'easing',
      'duration',
      'hover',
      'opacity',
      'reduced motion',
      'gif',
      'webp',
    ],
  },
  {
    frontend: 'go',
    slug: 'components',
    title: 'Components',
    description: 'Choose between primitives and accessible compound components.',
    outline: [
      { id: 'primitives', title: 'Primitives' },
      { id: 'compound-components', title: 'Compound components' },
      { id: 'controlled-state', title: 'Controlled state' },
      { id: 'component-families', title: 'Component families' },
      { id: 'styling-parts', title: 'Styling parts' },
    ],
    searchTerms: ['view', 'text', 'button', 'tabs', 'checkbox', 'unstyled'],
  },
  {
    frontend: 'go',
    slug: 'forms-and-input',
    title: 'Forms & Input',
    description: 'Build controlled fields, choices, and selection controls.',
    outline: [
      { id: 'text-input', title: 'Text input' },
      { id: 'fields', title: 'Fields' },
      { id: 'choices', title: 'Choices' },
      { id: 'select', title: 'Select' },
      { id: 'events', title: 'Events' },
    ],
    searchTerms: ['input', 'field', 'checkbox', 'radio', 'select', 'events'],
  },
  {
    frontend: 'go',
    slug: 'overlays-and-dialogs',
    title: 'Overlays & Dialogs',
    description: 'Present in-window overlays and operating-system dialogs.',
    outline: [
      { id: 'popover', title: 'Popover' },
      { id: 'dialog', title: 'Dialog' },
      { id: 'system-popover', title: 'System popover' },
      { id: 'native-dialogs', title: 'Native dialogs' },
    ],
    searchTerms: ['popover', 'dialog', 'overlay', 'alert', 'file picker'],
  },
  {
    frontend: 'go',
    slug: 'swift-ui',
    title: 'SwiftUI',
    description: 'Mount real SwiftUI controls inside a QuickGUI UI application.',
    outline: [
      { id: 'host', title: 'Host' },
      { id: 'native-controls', title: 'Native controls' },
      { id: 'controlled-values', title: 'Controlled values' },
      { id: 'sizing', title: 'Sizing' },
      { id: 'platform-support', title: 'Platform support' },
    ],
    searchTerms: ['swiftui', 'host', 'slider', 'toggle', 'picker', 'native'],
  },
  {
    frontend: 'go',
    slug: 'swift-ui-hosting',
    title: 'Modifiers & Hosting',
    description: 'Style SwiftUI controls and host QuickGUI content back inside them.',
    outline: [
      { id: 'modifiers', title: 'Modifiers' },
      { id: 'liquid-glass', title: 'Liquid Glass' },
      { id: 'reverse-hosting', title: 'Reverse hosting' },
      { id: 'swiftui-popover', title: 'SwiftUI popover' },
    ],
    searchTerms: ['modifier', 'glass', 'quickguihostview', 'popover', 'reverse host'],
  },
]

export function isDocsFrontend(value: string | undefined): value is DocsFrontend {
  return value === 'go' || value === 'moonbit'
}

export function docsPages(frontend: DocsFrontend): readonly DocsPageMeta[] {
  return frontend === 'go' ? GO_DOCS_PAGES : MOONBIT_DOCS_PAGES
}

export function docsPath(frontend: DocsFrontend, slug: DocsSlug = 'getting-started'): string {
  const root = `/docs/${frontend}`
  return slug === 'getting-started' ? root : `${root}/${slug}`
}

export function findDocsPage(frontend: DocsFrontend, slug?: string): DocsPageMeta | undefined {
  const normalized = slug || 'getting-started'
  return docsPages(frontend).find((page) => page.slug === normalized)
}

// Preserve the current guide or component when switching language frontends.
export function switchDocsFrontend(path: string, frontend: DocsFrontend): string {
  const [, , , slug, component] = path.split('/')
  if (component && (slug === 'components' || slug === 'swift-ui')) {
    return `/docs/${frontend}/${slug}/${component}`
  }
  const page = findDocsPage(frontend, slug)
  return docsPath(frontend, page?.slug)
}
