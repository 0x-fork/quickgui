import {
  HeadContent,
  Scripts,
  createRootRouteWithContext,
  useParams,
} from '@tanstack/react-router'
import type { QueryClient } from '@tanstack/react-query'
import { site } from '../lib/site'
import { htmlLang, resolveLocale } from '../i18n'
import { getOrigin } from '../server/origin'

import appCss from '../styles.css?url'

interface RouterContext {
  queryClient: QueryClient
}

export const Route = createRootRouteWithContext<RouterContext>()({
  loader: () => getOrigin(),
  head: ({ loaderData }) => {
    const origin = loaderData ?? ''
    return {
      meta: [
        { charSet: 'utf-8' },
        { name: 'viewport', content: 'width=device-width, initial-scale=1' },
        { title: site.name },
        { name: 'theme-color', content: '#0a0a0a' },
        { property: 'og:site_name', content: site.name },
        { property: 'og:type', content: 'website' },
        { property: 'og:image', content: `${origin}/og.png` },
        { name: 'twitter:card', content: 'summary_large_image' },
        { name: 'twitter:image', content: `${origin}/og.png` },
      ],
      links: [
        { rel: 'stylesheet', href: appCss },
        { rel: 'icon', type: 'image/svg+xml', href: '/favicon.svg' },
      ],
    }
  },
  notFoundComponent: NotFound,
  shellComponent: RootDocument,
})

function NotFound() {
  return (
    <div className="flex min-h-screen flex-col items-center justify-center gap-4">
      <p className="font-mono text-xs tracking-[0.22em] text-peach uppercase">
        404
      </p>
      <h1 className="text-2xl font-semibold tracking-tight">Page not found</h1>
      <a
        href="/"
        className="text-sm text-muted-foreground underline underline-offset-4 transition-colors hover:text-foreground"
      >
        Back to home
      </a>
    </div>
  )
}

function RootDocument({ children }: { children: React.ReactNode }) {
  const params = useParams({ strict: false }) as { locale?: string }
  const locale = resolveLocale(params.locale) ?? 'en'

  return (
    <html lang={htmlLang(locale)} className="dark" suppressHydrationWarning>
      <head>
        <HeadContent />
      </head>
      <body>
        {children}
        <Scripts />
      </body>
    </html>
  )
}
