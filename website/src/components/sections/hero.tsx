import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { CopyButton } from '../copy-button'
import { site } from '../../lib/site'
import type { RepoStats } from '../../lib/stats'

export function Hero({ stats }: { stats: RepoStats }) {
  const { t } = useTranslation()
  const version = stats.version ?? site.version

  return (
    <section className="border-b border-border">
      <div className="px-6 py-24 sm:px-12 sm:py-32">
        <a
          href={site.links.crate}
          target="_blank"
          rel="noreferrer"
          className="animate-fade-up inline-flex items-center gap-2 border border-border bg-card-2 px-3 py-1.5 font-mono text-xs text-muted-foreground transition-colors hover:text-foreground"
        >
          <span className="size-1.5 animate-pulse rounded-full bg-mint" />
          {t('hero.badge', { version })}
          <span className="i-lucide-arrow-up-right size-3" aria-hidden />
        </a>

        <h1 className="animate-fade-up mt-8 max-w-4xl text-5xl leading-[1.08] font-semibold tracking-tight text-balance [animation-delay:60ms] sm:text-6xl">
          {t('hero.title')}
        </h1>

        <p className="animate-fade-up mt-6 max-w-2xl text-base leading-relaxed text-muted-foreground [animation-delay:120ms] sm:text-lg">
          {t('hero.sub')}
        </p>

        <div className="animate-fade-up mt-10 flex flex-wrap items-center gap-3 [animation-delay:180ms]">
          <Button asChild className="h-10 gap-2 px-5 text-sm">
            <a href="#quickstart">
              {t('common.getStarted')}
              <span className="i-lucide-arrow-right size-4" aria-hidden />
            </a>
          </Button>
          <div className="flex h-10 items-center gap-2.5 border border-border bg-card-2 pr-1.5 pl-3.5 font-mono text-sm">
            <span className="text-peach select-none">$</span>
            <span>cargo add quickgui</span>
            <CopyButton text="cargo add quickgui" className="size-8" />
          </div>
        </div>
      </div>
    </section>
  )
}
