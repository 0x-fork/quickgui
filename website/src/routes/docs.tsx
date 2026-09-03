import type { Route } from './+types/docs'
import docsCss from '../docs.css?url'
import { DocsShell, type DocsArea } from '../components/docs/docs-shell'
import { getDocsMdxComponents } from '../components/docs/mdx-components'
import {
  OG_LOCALES,
  SUPPORTED_LOCALES,
  resolveLocale,
  type Locale,
} from '../i18n'
import { findDocsPage, docsPath, type DocsSlug } from '../lib/docs'
import { localizedDocsPage } from '../lib/docs-locales'
import { guideMdx } from '../lib/docs-mdx'
import { siteMeta } from '../lib/meta'

function localizedPath(locale: Locale, path: string): string {
  return locale === 'en' ? path : `/${locale}${path}`
}

function docsArea(slug: DocsSlug): DocsArea {
  if (slug === 'components' || slug === 'forms-and-input' || slug === 'overlays-and-dialogs') {
    return 'components'
  }
  if (slug === 'swift-ui' || slug === 'swift-ui-hosting') return 'swift-ui'
  return 'guide'
}

export function loader({ params, request }: Route.LoaderArgs) {
  const locale = resolveLocale(params.locale)
  const page = findDocsPage(params.slug)
  if (!locale || !page) throw new Response('Page not found', { status: 404 })

  return {
    locale,
    origin: new URL(request.url).origin,
    slug: page.slug,
  }
}

export const links: Route.LinksFunction = () => [
  { rel: 'stylesheet', href: docsCss },
]

export const meta: Route.MetaFunction = ({ loaderData }) => {
  const source = findDocsPage(loaderData?.slug)
  if (!source || !loaderData) return []

  const { locale, origin } = loaderData
  const page = localizedDocsPage(source, locale)
  const path = docsPath(page.slug)
  const title = `${page.title} | QuickGUI`

  return [
    ...siteMeta(origin, title),
    { name: 'description', content: page.description },
    { property: 'og:title', content: title },
    { property: 'og:description', content: page.description },
    { property: 'og:locale', content: OG_LOCALES[locale] },
    {
      tagName: 'link',
      rel: 'canonical',
      href: `${origin}${localizedPath(locale, path)}`,
    },
    ...SUPPORTED_LOCALES.map((other) => ({
      tagName: 'link' as const,
      rel: 'alternate',
      hrefLang: other,
      href: `${origin}${localizedPath(other, path)}`,
    })),
    {
      tagName: 'link',
      rel: 'alternate',
      hrefLang: 'x-default',
      href: `${origin}${path}`,
    },
  ]
}

export default function DocsRoute({ loaderData }: Route.ComponentProps) {
  const source = findDocsPage(loaderData.slug)
  if (!source) return null
  const page = localizedDocsPage(source, loaderData.locale)

  const Content = guideMdx(source.slug, loaderData.locale)
  return (
    <DocsShell
      locale={loaderData.locale}
      page={{
        title: page.title,
        description: page.description,
        outline: page.outline,
        path: docsPath(page.slug),
        area: docsArea(page.slug),
      }}
    >
      <Content components={getDocsMdxComponents(loaderData.locale)} />
    </DocsShell>
  )
}
