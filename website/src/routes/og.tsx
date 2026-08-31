import type { Route } from './+types/og'
import { Cross } from '../components/cross'
import { Logo } from '../components/logo'
import { siteMeta } from '../lib/meta'

// Renders the 1200×630 social card; public/og.png is a screenshot of this
// route. Not linked from anywhere and marked noindex.
export function loader({ request }: Route.LoaderArgs) {
  return { origin: new URL(request.url).origin }
}

export const meta: Route.MetaFunction = ({ loaderData }) => [
  ...siteMeta(loaderData?.origin ?? ''),
  { name: 'robots', content: 'noindex' },
]

export default function OgCard() {
  return (
    <div className="relative flex h-[630px] w-[1200px] flex-col justify-between overflow-hidden bg-background p-24">
      <div className="absolute inset-12 border border-border">
        <Cross className="top-0 left-0" />
        <Cross className="top-0 left-full" />
        <Cross className="top-full left-0" />
        <Cross className="top-full left-full" />
      </div>

      <div className="relative flex items-center gap-4">
        <Logo className="size-11" />
        <span className="text-3xl font-semibold tracking-tight">QuickGUI</span>
      </div>

      <div className="relative">
        <h1 className="max-w-4xl text-[64px] leading-[1.08] font-semibold tracking-tight">
          Desktop UI that only renders what changed.
        </h1>
        <p className="mt-8 font-mono text-xl text-muted-foreground">
          GPU-accelerated · Rust or TypeScript · zero CPU when idle
        </p>
      </div>

      <div className="relative flex items-center justify-between font-mono text-lg text-muted-foreground/80">
        <span>github.com/egoist/quickgui</span>
        <span>
          <span className="text-peach">$</span> cargo add quickgui
        </span>
      </div>
    </div>
  )
}
