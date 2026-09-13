import {
  isRouteErrorResponse,
  Links,
  Meta,
  Outlet,
  Scripts,
  ScrollRestoration,
  useParams,
} from 'react-router'
import type { Route } from './+types/root'
import { htmlLang, resolveLocale } from './i18n'
import { siteMeta } from './lib/meta'

import appCss from './styles.css?url'

export function loader({ request }: Route.LoaderArgs) {
  return { origin: new URL(request.url).origin }
}

export const meta: Route.MetaFunction = ({ loaderData }) => {
  const origin = loaderData?.origin ?? ''
  return siteMeta(origin)
}

export const links: Route.LinksFunction = () => [
  { rel: 'stylesheet', href: appCss },
  { rel: 'icon', type: 'image/png', href: '/favicon.png' },
]

export function Layout({ children }: { children: React.ReactNode }) {
  const { locale: localeParam } = useParams<'locale'>()
  const locale = resolveLocale(localeParam) ?? 'en'

  return (
    <html lang={htmlLang(locale)} suppressHydrationWarning>
      <head>
        <meta charSet="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <Meta />
        <Links />
        {import.meta.env.PROD ? (
          <script
            defer
            src="https://u.egoist.dev/script.js"
            data-website-id="77aceb0f-88e7-4237-b08f-86d964ed9425"
          />
        ) : null}
      </head>
      <body>
        {children}
        <ScrollRestoration />
        <Scripts />
      </body>
    </html>
  )
}

export default function App() {
  return <Outlet />
}

export function ErrorBoundary({ error }: Route.ErrorBoundaryProps) {
  const isNotFound = isRouteErrorResponse(error) && error.status === 404

  return (
    <div className="flex min-h-screen flex-col items-center justify-center gap-4">
      <p className="font-mono text-xs tracking-[0.22em] text-peach uppercase">
        {isNotFound ? '404' : 'Error'}
      </p>
      <h1 className="text-2xl font-semibold tracking-tight">
        {isNotFound ? 'Page not found' : 'Something went wrong'}
      </h1>
      <a
        href="/"
        className="text-sm text-muted-foreground underline underline-offset-4 transition-colors hover:text-foreground"
      >
        Back to home
      </a>
    </div>
  )
}
