import type { ComponentType } from 'react'
import type { Locale } from '../i18n'
import type { ComponentDocKind } from './component-docs'
import type { DocsFrontend, DocsSlug } from './docs'

export type DocsMdxComponent = ComponentType<{
  components?: Readonly<Record<string, ComponentType<any>>>
}>

type DocsMdxModule = { default: DocsMdxComponent }

const modules = import.meta.glob<DocsMdxModule>('../content/docs/**/*.mdx', {
  eager: true,
})

function contentAt(path: string): DocsMdxComponent | undefined {
  return modules[path]?.default
}

export function guideMdx(frontend: DocsFrontend, slug: DocsSlug, locale: Locale): DocsMdxComponent {
  const localized = contentAt(`../content/docs/${frontend}/${locale}/${slug}.mdx`)
  const fallback = contentAt(`../content/docs/${frontend}/en/${slug}.mdx`)
  if (!localized && !fallback) throw new Error(`Missing MDX guide: ${slug}`)
  return localized ?? fallback!
}

export function componentMdx(
  frontend: DocsFrontend,
  kind: ComponentDocKind,
  slug: string,
  locale: Locale,
): DocsMdxComponent {
  const family = kind === 'swift-ui' ? 'swift-ui' : 'ui'
  const localized = contentAt(
    `../content/docs/${frontend}/components/${locale}/${family}/${slug}.mdx`,
  )
  const fallback = contentAt(`../content/docs/${frontend}/components/${family}/${slug}.mdx`)
  if (!localized && !fallback) {
    throw new Error(`Missing MDX component page: ${kind}/${slug}`)
  }
  return localized ?? fallback!
}
