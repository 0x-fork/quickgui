import { redirect } from 'react-router'
import { resolveDocsRoute } from '../lib/docs-routing'
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
  const route = resolveDocsRoute(params.frontend, params.slug)
  if (!locale || !route) throw new Response('Page not found', { status: 404 })
  if (route.kind === 'redirect') {
    return redirect(localizedPath(locale, route.path) + new URL(request.url).search, 308)
  }
  const page = route.page

  return {
    locale,
    origin: new URL(request.url).origin,
    slug: page.slug,
    frontend: page.frontend,
  }
}

export const links: Route.LinksFunction = () => [
  { rel: 'stylesheet', href: docsCss },
]

export const meta: Route.MetaFunction = ({ loaderData }) => {
  if (!loaderData) return []
  const source = findDocsPage(loaderData.frontend, loaderData.slug)
  if (!source) return []

  const { locale, origin } = loaderData
  const page = localizedDocsPage(source, locale)
  const path = docsPath(page.frontend, page.slug)
  const title = `${page.title} | QuickGUI ${page.frontend === 'go' ? 'Go' : 'MoonBit'}`

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
  const source = findDocsPage(loaderData.frontend, loaderData.slug)
  if (!source) return null
  const page = localizedDocsPage(source, loaderData.locale)

  const Content = guideMdx(source.frontend, source.slug, loaderData.locale)
  return (
    <DocsShell
      frontend={loaderData.frontend}
      locale={loaderData.locale}
      page={{
        title: page.title,
        description: page.description,
        outline: page.outline,
        path: docsPath(page.frontend, page.slug),
        area: docsArea(page.slug),
      }}
    >
      <Content components={getDocsMdxComponents(loaderData.locale)} />
    </DocsShell>
  )
}
