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
import {
  LOCALE_LABELS,
  SUPPORTED_LOCALES,
  localePath,
  type Locale,
} from '../i18n'

const NAV_ITEMS = [
  { key: 'nav.features', href: '#features' },
  { key: 'nav.code', href: '#code' },
  { key: 'nav.architecture', href: '#architecture' },
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
          className="flex h-8 items-center gap-1.5 rounded-md border border-border bg-secondary/40 px-2.5 font-mono text-xs text-foreground/90 transition-colors hover:border-foreground/25 hover:bg-secondary"
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

export function SiteHeader() {
  const { t, i18n } = useTranslation()
  const current = (i18n.language as Locale) ?? 'en'

  return (
    <header className="fixed inset-x-0 top-0 z-50 border-b border-border/70 bg-background/80 backdrop-blur-md">
      <div className="mx-auto flex h-14 w-full max-w-6xl items-center justify-between gap-4 px-6">
        <a href={localePath(current)} className="flex items-center gap-2.5">
          <Logo />
          <span className="text-[15px] font-semibold tracking-tight">
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
            href={site.links.docs}
            target="_blank"
            rel="noreferrer"
            className="flex items-center gap-1 transition-colors hover:text-foreground"
          >
            {t('nav.docs')}
            <span className="i-lucide-arrow-up-right size-3" aria-hidden />
          </a>
        </nav>

        <div className="flex items-center gap-2.5">
          <LanguageMenu />
          <GitHubButton />
          <Button asChild size="sm" className="hidden h-8 sm:inline-flex">
            <a href="#quickstart">{t('common.getStarted')}</a>
          </Button>
        </div>
      </div>
    </header>
  )
}
