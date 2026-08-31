import { useEffect, useState } from 'react'
import { createFileRoute, notFound } from '@tanstack/react-router'
import { I18nextProvider, useTranslation } from 'react-i18next'
import { SiteHeader } from '../../components/site-header'
import { SiteFooter } from '../../components/site-footer'
import { Hero } from '../../components/sections/hero'
import { Features } from '../../components/sections/features'
import { CodeShowcase } from '../../components/sections/code-showcase'
import { Quickstart } from '../../components/sections/quickstart'
import { Platforms } from '../../components/sections/platforms'
import { FinalCta } from '../../components/sections/final-cta'
import { getHighlightedSnippets } from '../../server/highlight'
import { getOrigin } from '../../server/origin'
import { repoStatsQueryOptions } from '../../lib/queries'
import {
  OG_LOCALES,
  SUPPORTED_LOCALES,
  createI18n,
  localePath,
  messages,
  resolveLocale,
} from '../../i18n'

export const Route = createFileRoute('/{-$locale}/')({
  beforeLoad: ({ params }) => {
    if (resolveLocale(params.locale) === null) throw notFound()
  },
  loader: async ({ context }) => {
    const [highlighted, origin] = await Promise.all([
      getHighlightedSnippets(),
      getOrigin(),
      context.queryClient.prefetchQuery(repoStatsQueryOptions()),
    ])
    return { highlighted, origin }
  },
  head: ({ loaderData, params }) => {
    const locale = resolveLocale(params.locale) ?? 'en'
    const origin = loaderData?.origin ?? ''
    const meta = messages[locale].meta
    return {
      meta: [
        { title: meta.title },
        { name: 'description', content: meta.description },
        { property: 'og:title', content: meta.title },
        { property: 'og:description', content: meta.description },
        { property: 'og:locale', content: OG_LOCALES[locale] },
        ...SUPPORTED_LOCALES.filter((other) => other !== locale).map(
          (other) => ({
            property: 'og:locale:alternate',
            content: OG_LOCALES[other],
          }),
        ),
      ],
      links: [
        { rel: 'canonical', href: `${origin}${localePath(locale)}` },
        ...SUPPORTED_LOCALES.map((other) => ({
          rel: 'alternate',
          hrefLang: other,
          href: `${origin}${localePath(other)}`,
        })),
        { rel: 'alternate', hrefLang: 'x-default', href: `${origin}/` },
      ],
    }
  },
  component: Home,
})

function SkipLink() {
  const { t } = useTranslation()
  return (
    <a
      href="#features"
      className="sr-only focus:not-sr-only focus:fixed focus:top-2 focus:left-2 focus:z-[60] focus:bg-primary focus:px-3 focus:py-2 focus:text-sm focus:text-primary-foreground"
    >
      {t('common.skipToContent')}
    </a>
  )
}

function Home() {
  const { highlighted } = Route.useLoaderData()
  const params = Route.useParams()
  const locale = resolveLocale(params.locale) ?? 'en'
  const [i18n] = useState(() => createI18n(locale))

  useEffect(() => {
    if (i18n.language !== locale) void i18n.changeLanguage(locale)
  }, [i18n, locale])

  return (
    <I18nextProvider i18n={i18n}>
      <SkipLink />
      <SiteHeader />
      <main>
        {/* One bordered column runs the whole page; sections stack flush,
            separated by hairlines. */}
        <div className="mx-auto w-full max-w-6xl border-x border-border">
          <Hero />
          <Features />
          <CodeShowcase highlighted={highlighted} />
          <Quickstart highlighted={highlighted} />
          <Platforms />
          <FinalCta />
        </div>
      </main>
      <SiteFooter />
    </I18nextProvider>
  )
}
