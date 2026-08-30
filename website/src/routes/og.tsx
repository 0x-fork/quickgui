import { createFileRoute } from '@tanstack/react-router'
import { Logo } from '../components/logo'

// Renders the 1200×630 social card; public/og.png is a screenshot of this
// route. Not linked from anywhere and marked noindex.
export const Route = createFileRoute('/og')({
  head: () => ({ meta: [{ name: 'robots', content: 'noindex' }] }),
  component: OgCard,
})

function OgCard() {
  return (
    <div className="relative flex h-[630px] w-[1200px] flex-col justify-between overflow-hidden bg-background p-20">
      <div
        aria-hidden
        className="bg-dots absolute inset-0 [mask-image:radial-gradient(ellipse_70%_70%_at_50%_0%,black,transparent)]"
      />
      <div
        aria-hidden
        className="absolute -top-64 left-1/2 h-[500px] w-[900px] -translate-x-1/2 rounded-full bg-peach/[0.09] blur-[140px]"
      />

      <div className="relative flex items-center gap-4">
        <Logo className="size-12" />
        <span className="text-4xl font-semibold tracking-tight">QuickGUI</span>
      </div>

      <div className="relative">
        <h1 className="max-w-4xl bg-gradient-to-b from-white to-white/60 bg-clip-text text-7xl leading-[1.05] font-semibold tracking-tighter text-transparent">
          Desktop UI that only renders what changed.
        </h1>
        <p className="mt-8 font-mono text-2xl text-muted-foreground">
          damage-driven · GPU-accelerated · Rust
        </p>
      </div>

      <div className="relative flex items-center justify-between font-mono text-lg text-muted-foreground/60">
        <span>github.com/egoist/quickgui</span>
        <span>$ cargo add quickgui</span>
      </div>
    </div>
  )
}
