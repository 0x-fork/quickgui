import { useTranslation } from 'react-i18next'
import { Card } from '../card'
import { Container } from '../container'
import { SectionHeading } from '../section-heading'
import { site } from '../../lib/site'

const PLATFORMS = [
  { name: 'macOS', dot: 'bg-mint', statusKey: 'accepted', bodyKey: 'macos' },
  { name: 'Windows', dot: 'bg-peach', statusKey: 'compiles', bodyKey: 'windows' },
  { name: 'Linux', dot: 'bg-peach', statusKey: 'compiles', bodyKey: 'linux' },
] as const

const ROADMAP = [
  { version: '0.1', key: 'm1', done: true },
  { version: '0.2', key: 'm2', done: false },
  { version: '0.3', key: 'm3', done: false },
  { version: '1.0', key: 'm4', done: false },
] as const

export function Platforms() {
  const { t } = useTranslation()

  return (
    <section>
      <Container>
        <SectionHeading
          index="05"
          eyebrow={t('platforms.eyebrow')}
          title={t('platforms.title')}
          lead={t('platforms.lead')}
        />

        <div className="mt-10 grid gap-3 lg:grid-cols-2">
          <Card className="divide-y divide-border/60 px-8 py-2">
            {PLATFORMS.map((platform) => (
              <div key={platform.name} className="py-6">
                <div className="flex items-center gap-2.5">
                  <span
                    className={`size-1.5 rounded-full ${platform.dot}`}
                    aria-hidden
                  />
                  <h3 className="text-sm font-medium">{platform.name}</h3>
                  <span className="font-mono text-[11px] text-muted-foreground/70">
                    {t(`platforms.status.${platform.statusKey}`)}
                  </span>
                </div>
                <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
                  {t(`platforms.items.${platform.bodyKey}`)}
                </p>
              </div>
            ))}
          </Card>

          <Card className="p-8 sm:p-10">
            <h3 className="font-mono text-xs tracking-[0.22em] text-muted-foreground/70 uppercase">
              {t('platforms.roadmapTitle')}
            </h3>
            <ol className="mt-6">
              {ROADMAP.map((item, index) => (
                <li
                  key={item.version}
                  className="relative flex gap-5 pb-8 last:pb-0"
                >
                  {index < ROADMAP.length - 1 ? (
                    <span
                      aria-hidden
                      className="absolute top-7 left-[13px] h-[calc(100%-1.25rem)] w-px bg-border/70"
                    />
                  ) : null}
                  <span
                    className={`flex size-7 shrink-0 items-center justify-center rounded-full border font-mono text-[10px] ${
                      item.done
                        ? 'border-mint/40 bg-mint/10 text-mint'
                        : 'border-border bg-secondary/40 text-muted-foreground'
                    }`}
                  >
                    {item.version}
                  </span>
                  <div className="pt-1">
                    <h4 className="text-sm font-medium">
                      {t(`platforms.roadmap.${item.key}.label`)}
                      {item.done ? (
                        <span className="ml-2 font-mono text-[11px] text-mint">
                          {t('platforms.shipped')}
                        </span>
                      ) : null}
                    </h4>
                    <p className="mt-1 text-sm leading-relaxed text-muted-foreground">
                      {t(`platforms.roadmap.${item.key}.detail`)}
                    </p>
                  </div>
                </li>
              ))}
            </ol>
            <a
              href={site.links.status}
              target="_blank"
              rel="noreferrer"
              className="mt-8 inline-flex items-center gap-1 text-sm text-foreground/90 transition-colors hover:text-peach"
            >
              {t('platforms.statusLink')}
              <span className="i-lucide-arrow-up-right size-3.5" aria-hidden />
            </a>
          </Card>
        </div>
      </Container>
    </section>
  )
}
