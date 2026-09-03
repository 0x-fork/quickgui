import type { Route } from './+types/docs-component'
import docsCss from '../docs.css?url'
import { DocsShell } from '../components/docs/docs-shell'
import { getDocsMdxComponents } from '../components/docs/mdx-components'
import {
  OG_LOCALES,
  SUPPORTED_LOCALES,
  resolveLocale,
  type Locale,
} from '../i18n'
import {
  componentDocsPath,
  findComponentDoc,
  type ComponentDocKind,
} from '../lib/component-docs'
import {
  localizedComponentDescription,
  localizedComponentOutline,
} from '../lib/docs-locales'
import { componentMdx } from '../lib/docs-mdx'
import { siteMeta } from '../lib/meta'

function localizedPath(locale: Locale, path: string): string {
  return locale === 'en' ? path : `/${locale}${path}`
}

function componentKind(family: string | undefined): ComponentDocKind | null {
  if (family === 'components') return 'solid'
  if (family === 'swift-ui') return 'swift-ui'
  return null
}

export function loader({ params, request }: Route.LoaderArgs) {
  const locale = resolveLocale(params.locale)
  const kind = componentKind(params.family)
  const component = kind ? findComponentDoc(kind, params.component) : undefined
  if (!locale || !kind || !component) {
    throw new Response('Page not found', { status: 404 })
  }

  return {
    component: component.slug,
    kind,
    locale,
    origin: new URL(request.url).origin,
  }
}

export const links: Route.LinksFunction = () => [
  { rel: 'stylesheet', href: docsCss },
]

export const meta: Route.MetaFunction = ({ loaderData }) => {
  if (!loaderData) return []
  const component = findComponentDoc(loaderData.kind, loaderData.component)
  if (!component) return []

  const { locale, origin } = loaderData
  const path = componentDocsPath(component)
  const title = `${component.name} | QuickGUI`
  const description = localizedComponentDescription(component, locale)

  return [
    ...siteMeta(origin, title),
    { name: 'description', content: description },
    { property: 'og:title', content: title },
    { property: 'og:description', content: description },
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

export default function DocsComponentRoute({ loaderData }: Route.ComponentProps) {
  const component = findComponentDoc(loaderData.kind, loaderData.component)
  if (!component) return null

  const Content = componentMdx(
    component.kind,
    component.slug,
    loaderData.locale,
  )
  return (
    <DocsShell
      locale={loaderData.locale}
      page={{
        title: component.name,
        description: localizedComponentDescription(
          component,
          loaderData.locale,
        ),
        outline: localizedComponentOutline(component, loaderData.locale),
        path: componentDocsPath(component),
        area: component.kind === 'swift-ui' ? 'swift-ui' : 'components',
      }}
    >
      <Content components={getDocsMdxComponents(loaderData.locale)} />
    </DocsShell>
  )
}
