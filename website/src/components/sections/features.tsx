import { useTranslation } from 'react-i18next'
import { Container } from '../container'
import { SectionHeading } from '../section-heading'

const FEATURES = [
  { icon: 'i-lucide-moon-star', key: 'sleeps' },
  { icon: 'i-lucide-zap', key: 'gpu' },
  { icon: 'i-lucide-layout-grid', key: 'layout' },
  { icon: 'i-lucide-type', key: 'text' },
  { icon: 'i-lucide-accessibility', key: 'a11y' },
  { icon: 'i-lucide-rows-3', key: 'virtual' },
] as const

const CHIP_KEYS = [
  'menus',
  'dialogs',
  'popovers',
  'select',
  'tabs',
  'collections',
  'dnd',
  'clipboard',
  'notifications',
  'tray',
  'motion',
  'shaders',
  'tests',
  'inspector',
] as const

export function Features() {
  const { t } = useTranslation()

  return (
    <section id="features">
      <Container>
        <div className="rounded-3xl border border-border/60 bg-[#0e0e10] p-6 sm:p-12">
          <SectionHeading
            index="01"
            eyebrow={t('features.eyebrow')}
            title={t('features.title')}
            lead={t('features.lead')}
          />

          <div className="mt-12 grid gap-3 md:grid-cols-2 lg:grid-cols-3">
            {FEATURES.map((feature) => (
              <article
                key={feature.key}
                className="rounded-xl border border-border/50 bg-card-2 p-7 transition-colors hover:border-peach/25"
              >
                <span
                  className={`${feature.icon} size-5 text-peach`}
                  aria-hidden
                />
                <h3 className="mt-4 text-[15px] font-medium">
                  {t(`features.items.${feature.key}.title`)}
                </h3>
                <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
                  {t(`features.items.${feature.key}.body`)}
                </p>
              </article>
            ))}
          </div>

          <div className="mt-10 flex flex-wrap items-center gap-x-2 gap-y-2">
            <span className="mr-1 font-mono text-xs tracking-[0.18em] text-muted-foreground/70 uppercase">
              {t('features.alsoInTheBox')}
            </span>
            {CHIP_KEYS.map((key) => (
              <span
                key={key}
                className="rounded-full border border-border/70 px-2.5 py-1 font-mono text-[11px] text-muted-foreground"
              >
                {t(`features.chips.${key}`)}
              </span>
            ))}
          </div>
        </div>
      </Container>
    </section>
  )
}
