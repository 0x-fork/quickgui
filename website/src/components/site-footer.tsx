import { useTranslation } from 'react-i18next'
import { site } from '../lib/site'

const LINKS = [
  { labelKey: 'nav.docs', href: site.links.docs },
  { label: 'GitHub', href: site.links.github },
  { label: 'crates.io', href: site.links.crate },
] as const

export function SiteFooter() {
  const { t } = useTranslation()

  return (
    <footer>
      <div className="rail-joints rail-joints-top mx-auto flex w-full max-w-6xl flex-col justify-between gap-4 border-x border-t border-border px-6 py-8 sm:flex-row sm:items-center sm:px-12">
        <span className="font-mono text-xs text-muted-foreground/80">
          © 2026 EGOIST · {t('footer.license')}
        </span>
        <nav className="flex items-center gap-6 text-sm text-muted-foreground">
          {LINKS.map((link) => (
            <a
              key={link.href}
              href={link.href}
              target="_blank"
              rel="noreferrer"
              className="transition-colors hover:text-foreground"
            >
              {'labelKey' in link ? t(link.labelKey) : link.label}
            </a>
          ))}
        </nav>
      </div>
    </footer>
  )
}
