import type { RawData, Results, ZBSearch } from 'zbsearch'
import type { Locale } from '../i18n'
import {
  DOCS_SEARCH_SCHEMA,
  type DocsSearchDocument,
} from './docs-search-schema'

export interface DocsSearchHit {
  id: string
  score: number
  document: DocsSearchDocument
}

interface LoadedSearchIndex {
  engine: typeof import('zbsearch')
  database: ZBSearch<typeof DOCS_SEARCH_SCHEMA>
}

const indexes = new Map<Locale, Promise<LoadedSearchIndex>>()

async function loadDocsSearch(locale: Locale): Promise<LoadedSearchIndex> {
  const cached = indexes.get(locale)
  if (cached) return cached

  const pending = Promise.all([
    import('zbsearch'),
    fetch(`/docs-search/${locale}.json`).then(async (response) => {
      if (!response.ok) {
        throw new Error(`Unable to load documentation search (${response.status})`)
      }
      return response.json() as Promise<RawData>
    }),
  ])
    .then(async ([engine, payload]) => {
      const database = engine.create({
        schema: DOCS_SEARCH_SCHEMA,
        language: 'multilingual',
      })
      await engine.loadAsync(database, payload)
      return { engine, database }
    })
    .catch((error: unknown) => {
      indexes.delete(locale)
      throw error
    })

  indexes.set(locale, pending)
  return pending
}

export function prefetchDocsSearch(locale: Locale): void {
  void loadDocsSearch(locale).catch(() => undefined)
}

function differsByAtMostOne(left: string, right: string): boolean {
  if (Math.abs(left.length - right.length) > 1) return false
  let leftIndex = 0
  let rightIndex = 0
  let edits = 0

  while (leftIndex < left.length && rightIndex < right.length) {
    if (left[leftIndex] === right[rightIndex]) {
      leftIndex += 1
      rightIndex += 1
      continue
    }

    edits += 1
    if (edits > 1) return false
    if (left.length > right.length) leftIndex += 1
    else if (right.length > left.length) rightIndex += 1
    else {
      leftIndex += 1
      rightIndex += 1
    }
  }

  if (leftIndex < left.length || rightIndex < right.length) edits += 1
  return edits <= 1
}

export async function searchDocs(
  locale: Locale,
  query: string,
  limit = 10,
): Promise<DocsSearchHit[]> {
  const term = query.trim()
  if (!term) return []

  const { engine, database } = await loadDocsSearch(locale)
  const common = {
    mode: 'fulltext' as const,
    term,
    properties: [
      'pageTitle',
      'section',
      'hierarchy',
      'content',
      'keywords',
    ] as Array<keyof typeof DOCS_SEARCH_SCHEMA>,
    boost: {
      pageTitle: 7,
      section: 5,
      hierarchy: 2.5,
      keywords: 2,
      content: 1,
    },
    limit: limit * 2,
  }

  const prefixResults = await engine.search(database, {
    ...common,
    prefix: true,
  }) as Results<DocsSearchDocument>
  const hits = [...prefixResults.hits]

  if (term.length >= 4 && hits.length < limit) {
    const fuzzyResults = await engine.search(database, {
      ...common,
      tolerance: 1,
    }) as Results<DocsSearchDocument>
    const known = new Set(hits.map((hit) => hit.id))
    for (const hit of fuzzyResults.hits) {
      if (!known.has(hit.id)) hits.push(hit)
    }
  }

  const pages = new Map<string, DocsSearchHit[]>()
  for (const hit of hits) {
    const page = hit.document.url.split('#', 1)[0]
    const pageHits = pages.get(page) ?? []
    pageHits.push(hit)
    pages.set(page, pageHits)
  }

  const normalizedTerm = term.toLocaleLowerCase()
  const diversified = [...pages.values()].map((pageHits) => {
    const overview = pageHits.find((hit) => !hit.document.section)
    const title = overview?.document.pageTitle.toLocaleLowerCase() ?? ''
    if (
      overview &&
      (title === normalizedTerm ||
        title.startsWith(normalizedTerm) ||
        differsByAtMostOne(title, normalizedTerm))
    ) {
      return overview
    }
    return pageHits[0]
  })

  return diversified.slice(0, limit)
}
