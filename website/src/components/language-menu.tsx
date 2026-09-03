import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import {
  LOCALE_LABELS,
  SUPPORTED_LOCALES,
  type Locale,
} from '../i18n'

export function LanguageMenu({
  locale,
  label,
  hrefForLocale,
}: {
  locale: Locale
  label: string
  hrefForLocale: (locale: Locale) => string
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          type="button"
          aria-label={label}
          className="language-menu-trigger flex h-8 items-center gap-1.5 border border-border bg-background px-2.5 font-mono text-xs text-foreground/90 transition-colors hover:bg-muted"
        >
          <span className="i-lucide-globe size-3.5" aria-hidden />
          {LOCALE_LABELS[locale]}
          <span
            className="i-lucide-chevron-down size-3 opacity-60"
            aria-hidden
          />
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="min-w-32">
        {SUPPORTED_LOCALES.map((candidate) => (
          <DropdownMenuItem key={candidate} asChild>
            <a
              href={hrefForLocale(candidate)}
              aria-current={candidate === locale ? 'page' : undefined}
              className="flex items-center justify-between gap-4"
            >
              {LOCALE_LABELS[candidate]}
              {candidate === locale ? (
                <span className="i-lucide-check size-3.5" aria-hidden />
              ) : null}
            </a>
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
