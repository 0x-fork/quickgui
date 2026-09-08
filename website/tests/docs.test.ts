import { describe, expect, test } from 'bun:test'
import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import GithubSlugger from 'github-slugger'
import { SUPPORTED_LOCALES } from '../src/i18n'
import { ALL_COMPONENT_DOCS, componentDocsPath } from '../src/lib/component-docs'
import { DOCS_FRONTENDS, docsPages, docsPath, switchDocsFrontend } from '../src/lib/docs'
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

  test('switches shared topics and falls back for frontend-specific pages', () => {
    expect(switchDocsFrontend('/docs/go/styling', 'moonbit')).toBe('/docs/moonbit/styling')
    expect(switchDocsFrontend('/docs/moonbit/ui', 'go')).toBe('/docs/go/ui')
    expect(switchDocsFrontend('/docs/go', 'moonbit')).toBe('/docs/moonbit')
    expect(switchDocsFrontend('/docs/moonbit', 'go')).toBe('/docs/go')
    expect(switchDocsFrontend('/docs/go/components/button', 'moonbit')).toBe(
      '/docs/moonbit/components/button',
    )
    expect(switchDocsFrontend('/docs/go/swift-ui', 'moonbit')).toBe('/docs/moonbit/swift-ui')
    expect(switchDocsFrontend('/docs/moonbit/native-services', 'go')).toBe('/docs/go')
  })
})

test('localized guides exist and MoonBit outlines and examples match their content', async () => {
  for (const frontend of DOCS_FRONTENDS) {
    for (const source of docsPages(frontend)) {
      let englishExamples: string[] = []
      for (const locale of SUPPORTED_LOCALES) {
        const content = await readFile(
          resolve(root, 'src/content/docs', frontend, locale, `${source.slug}.mdx`),
          'utf8',
        )
        if (frontend !== 'moonbit') continue
        const page = localizedDocsPage(source, locale)
        const slugger = new GithubSlugger()
        const headings = [...content.matchAll(/^## (.+)$/gm)].map((match) => ({
          id: slugger.slug(match[1]),
          title: match[1],
        }))
        expect(headings).toEqual(page.outline)
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
