import { componentDocsPath, findComponentDoc } from './component-docs'
import { docsPath, findDocsPage, isDocsFrontend, type DocsFrontend, type DocsPageMeta } from './docs'

type DocsRoute = { kind: 'page'; page: DocsPageMeta } | { kind: 'redirect'; path: string }

export function resolveDocsRoute(
  frontend?: string,
  slug?: string,
  preferredFrontend: DocsFrontend = 'go',
): DocsRoute | undefined {
  if (isDocsFrontend(frontend)) {
    const page = findDocsPage(frontend, slug)
    if (!page) return undefined
    return slug === 'getting-started' || (slug !== undefined && slug !== page.slug)
      ? { kind: 'redirect', path: docsPath(frontend, page.slug) }
      : { kind: 'page', page }
  }

  if (!frontend && !slug) return { kind: 'redirect', path: docsPath(preferredFrontend) }

  // Preserve links from before the frontend-specific documentation structure.
  if ((frontend === 'components' || frontend === 'swift-ui') && slug) {
    const component = findComponentDoc(frontend === 'components' ? 'ui' : 'swift-ui', slug)
    if (component) return { kind: 'redirect', path: componentDocsPath(component) }
  } else if (!slug) {
    const page = findDocsPage('go', frontend)
    if (page) return { kind: 'redirect', path: docsPath('go', page.slug) }
  }
  return undefined
}
