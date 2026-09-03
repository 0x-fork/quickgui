import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { GitHubButton } from './github-button'
import { Logo } from './logo'
import { site } from '../lib/site'
import type { RepoStats } from '../lib/stats'
import {
  LOCALE_LABELS,
  SUPPORTED_LOCALES,
  localePath,
  type Locale,
} from '../i18n'

const NAV_ITEMS = [
  { key: 'nav.features', href: '#features' },
  { key: 'nav.code', href: '#code' },
  { key: 'nav.quickstart', href: '#quickstart' },
] as const

function LanguageMenu() {
  const { t, i18n } = useTranslation()
  const current = (i18n.language as Locale) ?? 'en'

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          type="button"
          aria-label={t('common.language')}
          className="flex h-8 items-center gap-1.5 border border-border bg-background px-2.5 font-mono text-xs text-foreground/90 transition-colors hover:bg-muted"
        >
          <span className="i-lucide-globe size-3.5" aria-hidden />
          {LOCALE_LABELS[current]}
          <span
            className="i-lucide-chevron-down size-3 opacity-60"
            aria-hidden
          />
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="min-w-32">
        {SUPPORTED_LOCALES.map((locale) => (
          <DropdownMenuItem key={locale} asChild>
            <a
              href={localePath(locale)}
              aria-current={locale === current ? 'page' : undefined}
              className="flex items-center justify-between gap-4"
            >
              {LOCALE_LABELS[locale]}
              {locale === current ? (
                <span className="i-lucide-check size-3.5" aria-hidden />
              ) : null}
            </a>
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

export function SiteHeader({ stats }: { stats: RepoStats }) {
  const { t, i18n } = useTranslation()
  const current = (i18n.language as Locale) ?? 'en'

  return (
    <header className="sticky top-0 z-50 bg-background/90 backdrop-blur-sm">
      <div className="rail-joints rail-joints-bottom mx-auto flex h-16 w-full max-w-6xl items-center justify-between gap-4 border-x border-b border-border px-6 sm:px-12">
        <a href={localePath(current)} className="flex items-center gap-2.5">
          <Logo />
          <span className="text-sm font-semibold tracking-tight">
            {site.name}
          </span>
        </a>

        <nav className="hidden items-center gap-7 text-sm text-muted-foreground lg:flex">
          {NAV_ITEMS.map((item) => (
            <a
              key={item.href}
              href={item.href}
              className="transition-colors hover:text-foreground"
            >
              {t(item.key)}
            </a>
          ))}
          <a
            href={current === 'en' ? site.links.docs : `/${current}${site.links.docs}`}
            className="transition-colors hover:text-foreground"
          >
            {t('nav.docs')}
          </a>
        </nav>

        <div className="flex items-center gap-2">
          <LanguageMenu />
          <GitHubButton stats={stats} />
          <Button asChild size="sm" className="hidden h-8 px-3.5 sm:inline-flex">
            <a href="#quickstart">{t('common.getStarted')}</a>
          </Button>
        </div>
      </div>
    </header>
  )
}
