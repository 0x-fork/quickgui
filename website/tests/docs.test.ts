import { describe, expect, test } from 'bun:test'
import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import GithubSlugger from 'github-slugger'
import { SUPPORTED_LOCALES } from '../src/i18n'
import { ALL_COMPONENT_DOCS, UI_COMPONENTS, componentDocsPath } from '../src/lib/component-docs'
import { DOCS_FRONTENDS, docsPages, docsPath, switchDocsFrontend } from '../src/lib/docs'
import { docsNavGroups } from '../src/lib/docs-navigation'
import { DOCS_GUIDE_ORDER } from '../src/lib/docs-structure'
import { localizedDocsPage } from '../src/lib/docs-locales'
import { resolveDocsRoute } from '../src/lib/docs-routing'
import { searchDocs } from '../src/lib/docs-search'

const root = resolve(import.meta.dir, '..')

describe('frontend documentation routes', () => {
  test('defaults to Go and canonicalizes getting-started URLs', () => {
    expect(resolveDocsRoute()).toEqual({ kind: 'redirect', path: '/docs/go' })
    for (const frontend of DOCS_FRONTENDS) {
      expect(resolveDocsRoute(frontend)?.kind).toBe('page')
      expect(resolveDocsRoute(frontend, 'getting-started')).toEqual({
        kind: 'redirect',
        path: `/docs/${frontend}`,
      })
    }
    expect(resolveDocsRoute('unknown')).toBeUndefined()
    expect(resolveDocsRoute('moonbit', 'swift-ui')?.kind).toBe('page')
  })

  test('redirects every previous Go guide and component URL', () => {
    for (const page of docsPages('go')) {
      expect(resolveDocsRoute(page.slug)).toEqual({
        kind: 'redirect',
        path: docsPath('go', page.slug),
      })
    }
    for (const component of ALL_COMPONENT_DOCS) {
      const family = component.kind === 'ui' ? 'components' : 'swift-ui'
      expect(resolveDocsRoute(family, component.slug)).toEqual({
        kind: 'redirect',
        path: componentDocsPath(component),
      })
    }
    expect(resolveDocsRoute('components', 'missing')).toBeUndefined()
    expect(resolveDocsRoute('unknown', 'styling')).toBeUndefined()
  })

  test('switches shared topics and falls back for unknown pages', () => {
    expect(switchDocsFrontend('/docs/go/styling', 'moonbit')).toBe('/docs/moonbit/styling')
    expect(switchDocsFrontend('/docs/moonbit/ui', 'go')).toBe('/docs/go/rendering')
    expect(switchDocsFrontend('/docs/go', 'moonbit')).toBe('/docs/moonbit')
    expect(switchDocsFrontend('/docs/moonbit', 'go')).toBe('/docs/go')
    expect(switchDocsFrontend('/docs/go/components/button', 'moonbit')).toBe(
      '/docs/moonbit/components/button',
    )
    expect(switchDocsFrontend('/docs/go/swift-ui', 'moonbit')).toBe('/docs/moonbit/swift-ui')
    expect(switchDocsFrontend('/docs/moonbit/native-services', 'go')).toBe(
      '/docs/go/native-services',
    )
    expect(switchDocsFrontend('/docs/moonbit/missing', 'go')).toBe('/docs/go')
  })

  test('redirects the former UI guide to Rendering in either frontend', () => {
    for (const frontend of DOCS_FRONTENDS) {
      expect(resolveDocsRoute(frontend, 'ui')).toEqual({
        kind: 'redirect',
        path: `/docs/${frontend}/rendering`,
      })
      for (const slug of ['reactivity', 'rendering', 'components', 'routing', 'styling']) {
        expect(resolveDocsRoute(frontend, slug)?.kind).toBe('page')
        expect(switchDocsFrontend(`/docs/go/${slug}`, frontend)).toBe(`/docs/${frontend}/${slug}`)
      }
    }
    expect(resolveDocsRoute('ui')).toEqual({ kind: 'redirect', path: '/docs/go/rendering' })
  })
})

test('Go and MoonBit share guide order, localized sections, and sidebar structure', () => {
  const go = docsPages('go')
  const moonbit = docsPages('moonbit')
  expect(go.map((page) => page.slug)).toEqual(DOCS_GUIDE_ORDER)
  expect(moonbit.map((page) => page.slug)).toEqual(DOCS_GUIDE_ORDER)
  for (const locale of SUPPORTED_LOCALES) {
    for (let index = 0; index < go.length; index++) {
      expect(localizedDocsPage(go[index], locale).title).toBe(
        localizedDocsPage(moonbit[index], locale).title,
      )
      expect(localizedDocsPage(go[index], locale).outline).toEqual(
        localizedDocsPage(moonbit[index], locale).outline,
      )
      expect(switchDocsFrontend(docsPath('go', go[index].slug), 'moonbit')).toBe(
        docsPath('moonbit', go[index].slug),
      )
    }
    const structure = (frontend: 'go' | 'moonbit') =>
      docsNavGroups(locale, frontend).map((group) => ({
        id: group.id,
        title: group.title,
        titles: group.items.map((item) => item.title),
        paths: group.items.map((item) => item.path.replace(`/docs/${frontend}`, '')),
      }))
    expect(structure('go')).toEqual(structure('moonbit'))
  }
})

test('the sidebar separates concepts from one complete component reference', async () => {
  for (const frontend of DOCS_FRONTENDS) {
    for (const locale of SUPPORTED_LOCALES) {
      const groups = docsNavGroups(locale, frontend)
      const concepts = groups.find((group) => group.id === 'concepts')!
      expect(concepts.items.map((item) => item.path)).toEqual(
        ['reactivity', 'rendering', 'components', 'routing', 'styling', 'animations'].map(
          (slug) => `/docs/${frontend}/${slug}`,
        ),
      )
      const components = groups.filter((group) => group.id === 'components')
      expect(components).toHaveLength(1)
      expect(components[0].items.map((item) => item.path).sort()).toEqual(
        UI_COMPONENTS.map((component) => componentDocsPath(component, frontend)).sort(),
      )
      const names = components[0].items.map((item) => item.title)
      expect(names).toEqual([...names].sort((a, b) => a.localeCompare(b, 'en')))
      const paths = groups.flatMap((group) => group.items.map((item) => item.path))
      expect(new Set(paths).size).toBe(paths.length)
      if (locale === 'en') {
        expect(concepts.title).toBe('Concepts')
        expect(groups.map((group) => group.title)).not.toContain('Primitives')
        expect(groups.map((group) => group.title)).not.toContain('QuickGUI UI')
      }
      for (const component of UI_COMPONENTS) {
        const content = await readFile(
          resolve(
            root,
            'src/content/docs',
            frontend,
            'components',
            locale === 'en' ? '' : locale,
            'ui',
            `${component.slug}.mdx`,
          ),
          'utf8',
        )
        expect(content.trim().length).toBeGreaterThan(0)
      }
    }
  }
})

test('localized concepts and MoonBit guides keep matching outlines and examples', async () => {
  for (const frontend of DOCS_FRONTENDS) {
    for (const source of docsPages(frontend)) {
      let englishExamples: string[] = []
      for (const locale of SUPPORTED_LOCALES) {
        const content = await readFile(
          resolve(root, 'src/content/docs', frontend, locale, `${source.slug}.mdx`),
          'utf8',
        )
        const page = localizedDocsPage(source, locale)
        const slugger = new GithubSlugger()
        const headings = [...content.matchAll(/^## (.+)$/gm)].map((match) => ({
          id: slugger.slug(match[1]),
          title: match[1],
        }))
        expect(headings).toEqual(page.outline)
        if (
          frontend !== 'moonbit' &&
          !['reactivity', 'rendering', 'components', 'routing', 'styling'].includes(source.slug)
        )
          continue
        const examples = [...content.matchAll(/```[^\n]*\n[\s\S]*?```/g)].map((match) => match[0])
        if (locale === 'en') englishExamples = examples
        else expect(examples).toEqual(englishExamples)
      }
    }
  }
})

test('all internal MDX links use valid frontend routes', async () => {
  const paths = new Set([
    ...DOCS_FRONTENDS.flatMap((frontend) =>
      docsPages(frontend).map((page) => docsPath(frontend, page.slug)),
    ),
    ...DOCS_FRONTENDS.flatMap((frontend) =>
      ALL_COMPONENT_DOCS.map((component) => componentDocsPath(component, frontend)),
    ),
  ])
  const contentRoot = resolve(root, 'src/content/docs')
  for await (const file of new Bun.Glob('**/*.mdx').scan(contentRoot)) {
    const content = await readFile(resolve(contentRoot, file), 'utf8')
    const links = [...content.matchAll(/\]\((\/docs[^\s)]*)\)/g)]
    for (const [, link] of links) {
      const path = link.split(/[?#]/, 1)[0]
      expect(paths.has(path), `${file}: ${link}`).toBe(true)
    }
  }
})

test('every Go component has a localized MoonBit reference and keeps its route when switching', async () => {
  for (const component of ALL_COMPONENT_DOCS) {
    const path = componentDocsPath(component, 'moonbit')
    expect(switchDocsFrontend(componentDocsPath(component), 'moonbit')).toBe(path)
    expect(switchDocsFrontend(path, 'go')).toBe(componentDocsPath(component))
    let example = ''
    for (const locale of SUPPORTED_LOCALES) {
      const file = resolve(
        root,
        'src/content/docs/moonbit/components',
        locale === 'en' ? '' : locale,
        component.kind,
        `${component.slug}.mdx`,
      )
      const content = await readFile(file, 'utf8')
      expect(content).not.toContain('```go')
      const code = [...content.matchAll(/```moonbit\n([\s\S]*?)```/g)]
        .map((match) => match[1])
        .join('\n')
      expect(code).toContain('fn component_example() -> @ui.Element')
      if (locale === 'en') example = code
      else expect(code).toBe(example)
    }
  }
})

test('search scopes results and caches by frontend and locale', async () => {
  const originalFetch = globalThis.fetch
  const requested: string[] = []
  globalThis.fetch = (async (input: string | URL | Request) => {
    const path = String(input)
    requested.push(path)
    return new Response(Bun.file(resolve(root, 'public', path.slice(1))))
  }) as typeof fetch
  try {
    for (const frontend of DOCS_FRONTENDS) {
      for (const locale of SUPPORTED_LOCALES) {
        const prefix = `${locale === 'en' ? '' : `/${locale}`}/docs/${frontend}`
        const hits = await searchDocs(locale, frontend, 'transition')
        expect(hits.length).toBeGreaterThan(0)
        expect(hits.every((hit) => hit.document.url.startsWith(prefix))).toBe(true)
      }
    }
    const moonbit = await searchDocs('en', 'moonbit', 'button')
    expect(moonbit.every((hit) => !hit.document.url.includes('/docs/go/'))).toBe(true)
    const checkbox = await searchDocs('en', 'moonbit', 'checkbox')
    expect(
      checkbox.some((hit) => hit.document.url.startsWith('/docs/moonbit/components/checkbox')),
    ).toBe(true)
    expect(requested.sort()).toEqual(
      DOCS_FRONTENDS.flatMap((frontend) =>
        SUPPORTED_LOCALES.map((locale) => `/docs-search/${frontend}/${locale}.json`),
      ).sort(),
    )
  } finally {
    globalThis.fetch = originalFetch
  }
})
