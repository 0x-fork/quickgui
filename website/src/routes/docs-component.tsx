import type { Route } from './+types/docs-component'
import docsCss from '../docs.css?url'
import { DocsShell } from '../components/docs/docs-shell'
import { getDocsMdxComponents } from '../components/docs/mdx-components'
import { OG_LOCALES, SUPPORTED_LOCALES, resolveLocale, type Locale } from '../i18n'
import { componentDocsPath, findComponentDoc, type ComponentDocKind } from '../lib/component-docs'
import { localizedComponentDescription, localizedComponentOutline } from '../lib/docs-locales'
import { componentMdx } from '../lib/docs-mdx'
import { isDocsFrontend, frontendLabel } from '../lib/docs'
import { getComponentApi } from '../lib/component-api.server'
import { getDemoSource } from '../lib/demo-source.server'
import { ComponentApiReference } from '../components/docs/component-api-reference'
import { ComponentPreview } from '../components/docs/component-preview'
import { DEMO_COMPONENTS } from '../lib/component-demos'
import { siteMeta } from '../lib/meta'

function localizedPath(locale: Locale, path: string): string {
  return locale === 'en' ? path : `/${locale}${path}`
}

function componentKind(family: string | undefined): ComponentDocKind | null {
  if (family === 'components') return 'ui'
  if (family === 'swift-ui') return 'swift-ui'
  return null
}

export function loader({ params, request }: Route.LoaderArgs) {
  const locale = resolveLocale(params.locale)
  const kind = componentKind(params.family)
  const component = kind ? findComponentDoc(kind, params.component) : undefined
  if (!isDocsFrontend(params.frontend) || !locale || !kind || !component) {
    throw new Response('Page not found', { status: 404 })
  }

  return {
    frontend: params.frontend,
    component: component.slug,
    kind,
    locale,
    origin: new URL(request.url).origin,
    api: getComponentApi(params.frontend, kind, component.slug),
    demoSource: kind === 'ui' ? getDemoSource(params.frontend, component.slug) : undefined,
  }
}

export const links: Route.LinksFunction = () => [{ rel: 'stylesheet', href: docsCss }]

export const meta: Route.MetaFunction = ({ loaderData }) => {
  if (!loaderData) return []
  const component = findComponentDoc(loaderData.kind, loaderData.component)
  if (!component) return []

  const { locale, origin } = loaderData
  const path = componentDocsPath(component, loaderData.frontend)
  const title = `${component.name} | QuickGUI ${frontendLabel(loaderData.frontend)}`
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
    loaderData.frontend,
    component.kind,
    component.slug,
    loaderData.locale,
  )
  return (
    <DocsShell
      frontend={loaderData.frontend}
      locale={loaderData.locale}
      page={{
        title: component.name,
        description: localizedComponentDescription(component, loaderData.locale),
        outline: localizedComponentOutline(component, loaderData.locale),
        path: componentDocsPath(component, loaderData.frontend),
        area: component.kind === 'swift-ui' ? 'swift-ui' : 'components',
      }}
    >
      <p className="docs-lead">{localizedComponentDescription(component, loaderData.locale)}</p>
      {component.kind === 'ui' && DEMO_COMPONENTS.includes(component.slug) && loaderData.demoSource && <ComponentPreview key={`${component.kind}/${component.slug}`} component={component} source={loaderData.demoSource} locale={loaderData.locale} />}
      <Content components={getDocsMdxComponents(loaderData.locale)} />
      <ComponentApiReference key={`${loaderData.frontend}/${component.slug}`} api={loaderData.api} locale={loaderData.locale} />
    </DocsShell>
  )
}
