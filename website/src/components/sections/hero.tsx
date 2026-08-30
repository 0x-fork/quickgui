import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Container } from '../container'
import { CopyButton } from '../copy-button'
import { CounterDemo } from '../counter-demo'
import { repoStatsQueryOptions } from '../../lib/queries'
import { site } from '../../lib/site'

export function Hero() {
  const { t } = useTranslation()
  const { data } = useQuery(repoStatsQueryOptions())
  const version = data?.version ?? site.version

  return (
    <section className="pt-32 sm:pt-36">
      <Container className="flex flex-col items-center text-center">
        <a
          href={site.links.crate}
          target="_blank"
          rel="noreferrer"
          className="animate-fade-up inline-flex items-center gap-2.5 rounded-md border border-border bg-secondary/30 px-3 py-1.5 font-mono text-xs text-muted-foreground transition-colors hover:border-foreground/25 hover:text-foreground"
        >
          <span className="size-1.5 animate-pulse rounded-full bg-mint" />
          {t('hero.badge', { version })}
          <span className="i-lucide-arrow-up-right size-3" aria-hidden />
        </a>

        <h1 className="animate-fade-up mt-8 max-w-4xl bg-gradient-to-b from-white to-white/60 bg-clip-text text-5xl leading-[1.03] font-semibold tracking-tighter text-transparent text-balance [animation-delay:60ms] sm:text-6xl lg:text-7xl">
          {t('hero.title')}
        </h1>

        <p className="animate-fade-up mt-6 max-w-xl text-lg leading-relaxed text-muted-foreground [animation-delay:120ms]">
          {t('hero.sub')}
        </p>

        <div className="animate-fade-up mt-9 flex flex-wrap items-center justify-center gap-3 [animation-delay:180ms]">
          <Button asChild size="lg" className="h-11 gap-2 px-5 text-[15px]">
            <a href="#quickstart">
              {t('common.getStarted')}
              <span className="i-lucide-arrow-right size-4" aria-hidden />
            </a>
          </Button>
          <div className="flex h-11 items-center gap-3 rounded-md border border-border bg-secondary/30 pr-1.5 pl-4 font-mono text-sm">
            <span className="text-peach select-none">$</span>
            <span>cargo add quickgui</span>
            <CopyButton text="cargo add quickgui" />
          </div>
        </div>
      </Container>

      <Container className="mt-14 sm:mt-16">
        <div className="animate-fade-up relative overflow-hidden rounded-3xl border border-border/60 bg-card px-4 py-12 [animation-delay:240ms] sm:px-14 sm:py-16">
          <div
            aria-hidden
            className="bg-dots absolute inset-0 [mask-image:radial-gradient(ellipse_75%_90%_at_50%_0%,black,transparent)]"
          />
          <div
            aria-hidden
            className="absolute -top-40 left-1/2 h-[380px] w-[680px] -translate-x-1/2 rounded-full bg-peach/[0.08] blur-[120px]"
          />
          <div className="relative mx-auto max-w-[540px]">
            <CounterDemo />
          </div>
          <p className="relative mt-5 text-center font-mono text-[11px] text-muted-foreground/60">
            {t('hero.caption')}
          </p>
        </div>
      </Container>
    </section>
  )
}
