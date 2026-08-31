import { useTranslation } from 'react-i18next'
import { SectionHeading } from '../section-heading'

const FEATURES = [
  'idle',
  'fast',
  'layout',
  'components',
  'text',
  'a11y',
  'lists',
  'native',
  'cli',
] as const

export function Features() {
  const { t } = useTranslation()

  return (
    <section id="features" className="border-b border-border">
      <SectionHeading title={t('features.title')} />

      <div className="grid gap-px bg-border sm:grid-cols-2 lg:grid-cols-3">
        {FEATURES.map((key) => (
          <article
            key={key}
            className="bg-background p-8 last:sm:col-span-2 last:lg:col-span-1 sm:p-10"
          >
            <h3 className="text-base font-medium">
              {t(`features.items.${key}.title`)}
            </h3>
            <p className="mt-3 text-[15px] leading-relaxed text-muted-foreground">
              {t(`features.items.${key}.body`)}
            </p>
          </article>
        ))}
      </div>
    </section>
  )
}
