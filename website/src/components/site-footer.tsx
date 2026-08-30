import { useTranslation } from 'react-i18next'
import { Container } from './container'
import { Logo } from './logo'
import { site } from '../lib/site'

const LINK_GROUPS = [
  {
    titleKey: 'footer.project',
    links: [
      { labelKey: 'footer.links.docs', href: site.links.docs },
      { labelKey: 'footer.links.viewApi', href: site.links.viewApi },
      { labelKey: 'footer.links.architecture', href: site.links.architecture },
      { labelKey: 'footer.links.status', href: site.links.status },
      { labelKey: 'footer.links.changelog', href: site.links.changelog },
    ],
  },
  {
    titleKey: 'footer.packages',
    links: [
      { labelKey: 'footer.links.crate', href: site.links.crate },
      { labelKey: 'footer.links.docsRs', href: site.links.docsRs },
      { labelKey: 'footer.links.solid', href: site.links.solid },
      { labelKey: 'footer.links.cli', href: site.links.cli },
    ],
  },
  {
    titleKey: 'footer.community',
    links: [
      { labelKey: 'footer.links.github', href: site.links.github },
      { labelKey: 'footer.links.issues', href: `${site.links.github}/issues` },
      { labelKey: 'footer.links.examples', href: site.links.examples },
      { labelKey: 'footer.links.license', href: site.links.license },
    ],
  },
] as const

export function SiteFooter() {
  const { t } = useTranslation()

  return (
    <footer className="border-t border-border/60">
      <Container className="grid gap-12 py-16 md:grid-cols-[1.2fr_repeat(3,minmax(0,1fr))]">
        <div>
          <div className="flex items-center gap-2.5">
            <Logo />
            <span className="text-[15px] font-semibold tracking-tight">
              {site.name}
            </span>
          </div>
          <p className="mt-4 max-w-xs text-sm leading-relaxed text-muted-foreground">
            {t('footer.description')}
          </p>
        </div>

        {LINK_GROUPS.map((group) => (
          <nav key={group.titleKey} aria-label={t(group.titleKey)}>
            <h3 className="font-mono text-xs tracking-[0.18em] text-muted-foreground/70 uppercase">
              {t(group.titleKey)}
            </h3>
            <ul className="mt-4 space-y-2.5 text-sm">
              {group.links.map((link) => (
                <li key={link.href}>
                  <a
                    href={link.href}
                    target="_blank"
                    rel="noreferrer"
                    className="text-muted-foreground transition-colors hover:text-foreground"
                  >
                    {t(link.labelKey)}
                  </a>
                </li>
              ))}
            </ul>
          </nav>
        ))}
      </Container>
      <div className="border-t border-border/60">
        <Container className="flex flex-col justify-between gap-2 py-6 font-mono text-xs text-muted-foreground/60 sm:flex-row">
          <span>© 2026 EGOIST</span>
          <span>{t('footer.license')}</span>
        </Container>
      </div>
    </footer>
  )
}
