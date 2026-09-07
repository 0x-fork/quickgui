export type DocsSlug =
  | 'getting-started'
  | 'project-structure'
  | 'ui'
  | 'styling'
  | 'animations'
  | 'components'
  | 'forms-and-input'
  | 'overlays-and-dialogs'
  | 'swift-ui'
  | 'swift-ui-hosting'

export interface DocsOutlineItem {
  id: string
  title: string
  level?: 2 | 3
}

export interface DocsPageMeta {
  slug: DocsSlug
  title: string
  description: string
  outline: readonly DocsOutlineItem[]
  searchTerms: readonly string[]
}

export interface DocsNavGroup {
  title: string
  items: readonly DocsSlug[]
}

export const DOCS_PAGES: readonly DocsPageMeta[] = [
  {
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
    searchTerms: ['transition', 'animation', 'easing', 'duration', 'hover', 'opacity', 'reduced motion', 'gif', 'webp'],
  },
  {
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

export const DOCS_NAV: readonly DocsNavGroup[] = [
  {
    title: 'Introduction',
    items: ['getting-started', 'project-structure'],
  },
  {
    title: 'QuickGUI UI',
    items: ['ui', 'styling', 'animations'],
  },
  {
    title: 'Components',
    items: ['components', 'forms-and-input', 'overlays-and-dialogs'],
  },
  {
    title: 'SwiftUI',
    items: ['swift-ui', 'swift-ui-hosting'],
  },
]

export function docsPath(slug: DocsSlug): string {
  return slug === 'getting-started' ? '/docs' : `/docs/${slug}`
}

export function findDocsPage(slug?: string): DocsPageMeta | undefined {
  const normalized = slug || 'getting-started'
  return DOCS_PAGES.find((page) => page.slug === normalized)
}

export function docsPageIndex(slug: DocsSlug): number {
  return DOCS_PAGES.findIndex((page) => page.slug === slug)
}
