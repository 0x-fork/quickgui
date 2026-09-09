import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'
import GithubSlugger from 'github-slugger'
import { unified } from 'unified'
import remarkGfm from 'remark-gfm'
import remarkMdx from 'remark-mdx'
import remarkParse from 'remark-parse'
import { create, insertMultiple, save } from 'zbsearch'
import { getComponentApi } from '../src/lib/component-api.server'
import { COMPONENT_DOC_LABELS } from '../src/lib/docs-locales'
import { ALL_COMPONENT_DOCS, componentDocsPath, type ComponentDoc } from '../src/lib/component-docs'
import {
  DOCS_FRONTENDS,
  docsPages,
  docsPath,
  type DocsFrontend,
  type DocsPageMeta,
} from '../src/lib/docs'
import { localizedComponentDescription, localizedDocsPage } from '../src/lib/docs-locales'
import {
  DOCS_SEARCH_SCHEMA,
  type DocsSearchArea,
  type DocsSearchDocument,
} from '../src/lib/docs-search-schema'
import { SUPPORTED_LOCALES, type Locale } from '../src/i18n'

type MarkdownNode = {
  type: string
  value?: string
  depth?: number
  children?: MarkdownNode[]
}

interface SearchPage {
  title: string
  description: string
  keywords: readonly string[]
  area: DocsSearchArea
  path: string
  sourcePath: string
  fallbackPath: string
}

const websiteRoot = resolve(import.meta.dir, '..')
const contentRoot = resolve(websiteRoot, 'src/content/docs')
const outputRoot = resolve(websiteRoot, 'public/docs-search')

const parser = unified().use(remarkParse).use(remarkMdx).use(remarkGfm)

function localizedPath(locale: Locale, path: string): string {
  return locale === 'en' ? path : `/${locale}${path}`
}

function guideSource(page: DocsPageMeta, locale: Locale): string {
  const localized = resolve(contentRoot, page.frontend, locale, `${page.slug}.mdx`)
  const fallback = resolve(contentRoot, page.frontend, 'en', `${page.slug}.mdx`)
  return locale === 'en' ? fallback : localized
}

function componentSource(component: ComponentDoc, locale: Locale, frontend: DocsFrontend): string {
  const family = component.kind === 'swift-ui' ? 'swift-ui' : 'ui'
  const localized = resolve(
    contentRoot,
    frontend,
    'components',
    locale,
    family,
    `${component.slug}.mdx`,
  )
  const fallback = resolve(contentRoot, frontend, 'components', family, `${component.slug}.mdx`)
  return locale === 'en' ? fallback : localized
}

async function readableSource(preferred: string, fallback: string): Promise<string> {
  try {
    return await readFile(preferred, 'utf8')
  } catch (error) {
    if (
      preferred !== fallback &&
      error instanceof Error &&
      'code' in error &&
      error.code === 'ENOENT'
    ) {
      return readFile(fallback, 'utf8')
    }
    throw error
  }
}

function pageDefinitions(locale: Locale, frontend: DocsFrontend): SearchPage[] {
  const guides: SearchPage[] = docsPages(frontend).map((source) => {
    const page = localizedDocsPage(source, locale)
    return {
      title: page.title,
      description: page.description,
      keywords: page.searchTerms,
      area:
        page.slug === 'swift-ui' || page.slug === 'swift-ui-hosting'
          ? 'swift-ui'
          : 'guide',
      path: docsPath(frontend, page.slug),
      sourcePath: guideSource(page, locale),
      fallbackPath: guideSource(page, 'en'),
    }
  })

  const components: SearchPage[] = ALL_COMPONENT_DOCS.map((component) => ({
    title: component.name,
    description: localizedComponentDescription(component, locale),
    keywords: [component.section, ...component.parts, ...component.keyProps],
    area: component.kind,
    path: componentDocsPath(component, frontend),
    sourcePath: componentSource(component, locale, frontend),
    fallbackPath: componentSource(component, 'en', frontend),
  }))

  return [...guides, ...components]
}

function nodeText(node: MarkdownNode): string {
  if (
    node.type === 'code' ||
    node.type === 'html' ||
    node.type === 'mdxjsEsm' ||
    node.type === 'mdxFlowExpression' ||
    node.type === 'mdxTextExpression'
  ) {
    return ''
  }

  if (node.type === 'text' || node.type === 'inlineCode') {
    return node.value ?? ''
  }

  return node.children?.map(nodeText).filter(Boolean).join(' ') ?? ''
}

function normalizeText(value: string): string {
  return value.replace(/\s+/g, ' ').trim()
}

function recordsForPage(page: SearchPage, source: string, locale: Locale): DocsSearchDocument[] {
  const tree = parser.parse({ value: source, path: page.sourcePath }) as MarkdownNode
  const slugger = new GithubSlugger()
  const records: DocsSearchDocument[] = []
  let parentHeading = ''
  let section = ''
  let hierarchy = page.title
  let anchor = ''
  let body: string[] = [page.description]

  function flush() {
    const content = normalizeText(body.join(' '))
    const url = `${localizedPath(locale, page.path)}${anchor ? `#${anchor}` : ''}`
    records.push({
      id: `${locale}:${page.path}:${anchor || 'overview'}`,
      pageTitle: page.title,
      section,
      hierarchy,
      content,
      keywords: section ? '' : page.keywords.join(' '),
      url,
      area: page.area,
    })
  }

  for (const child of tree.children ?? []) {
    if (child.type === 'heading' && (child.depth === 2 || child.depth === 3)) {
      flush()
      section = normalizeText(nodeText(child))
      anchor = slugger.slug(section)
      if (child.depth === 2) parentHeading = section
      hierarchy =
        child.depth === 3 && parentHeading
          ? `${page.title} › ${parentHeading} › ${section}`
          : `${page.title} › ${section}`
      body = []
      continue
    }

    const text = normalizeText(nodeText(child))
    if (text && body[body.length - 1] !== text) body.push(text)
  }

  flush()
  return records
}

async function buildLocale(locale: Locale, frontend: DocsFrontend) {
  const records: DocsSearchDocument[] = []

  for (const page of pageDefinitions(locale, frontend)) {
    let source = await readableSource(page.sourcePath, page.fallbackPath)
    const component = ALL_COMPONENT_DOCS.find(component => componentDocsPath(component, frontend) === page.path)
    if (component) {
      // Match the rendered route: the summary table is replaced by the complete API.
      source = source.split(`## ${COMPONENT_DOC_LABELS[locale].keyProps}`)[0]
      const api = getComponentApi(frontend, component.kind, component.slug)
      const content = api.sections.map(section =>
        section.name + ' ' + section.signature + '\n' + section.entries.map(entry => `${entry.name} ${entry.type} ${entry.description}`).join('\n'),
      ).join('\n')
      // Signatures are literal text, not MDX: generics and object types can contain <>{}.
      records.push({ id: `${locale}:${page.path}:api-reference`, pageTitle: page.title,
        section: 'API reference', hierarchy: `${page.title} › API reference`, content: normalizeText(content),
        keywords: '', url: `${localizedPath(locale, page.path)}#api-reference`, area: page.area })
    }
    records.push(...recordsForPage(page, source, locale))
  }

  const database = create({
    schema: DOCS_SEARCH_SCHEMA,
    language: 'multilingual',
  })
  insertMultiple(database, records)

  const payload = save(database)
  const destination = resolve(outputRoot, frontend, `${locale}.json`)
  await mkdir(dirname(destination), { recursive: true })
  const serialized = JSON.stringify(payload)
  if (!(await Bun.file(destination).exists()) || await readFile(destination, 'utf8') !== serialized) {
    await writeFile(destination, serialized)
  }

  return { locale, frontend, records: records.length }
}

const summaries = await Promise.all(
  DOCS_FRONTENDS.flatMap((frontend) =>
    SUPPORTED_LOCALES.map((locale) => buildLocale(locale, frontend)),
  ),
)
console.log(
  `Built documentation search indexes (${summaries
    .map(({ locale, frontend, records }) => `${frontend}/${locale}: ${records}`)
    .join(', ')})`,
)
