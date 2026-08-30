import { useTranslation } from 'react-i18next'
import { Container } from '../container'
import { SectionHeading } from '../section-heading'
import { VirtualListDemo } from '../virtual-list-demo'
import { site } from '../../lib/site'

const PIPELINE = [
  { step: '01', key: 'input' },
  { step: '02', key: 'tree' },
  { step: '03', key: 'layout' },
  { step: '04', key: 'scene' },
  { step: '05', key: 'submit' },
] as const

const GATE_NUMBERS = [
  { key: 'scroll', value: '95.05 Hz' },
  { key: 'frameCpu', value: '1.3496 ms' },
  { key: 'processCpu', value: '14.65 %' },
  { key: 'rss', value: '106.9 MiB' },
  { key: 'idle', value: '0' },
] as const

export function Architecture() {
  const { t } = useTranslation()

  return (
    <section id="architecture">
      <Container>
        <div className="rounded-3xl border border-border/60 bg-card p-6 sm:p-12">
          <SectionHeading
            index="03"
            eyebrow={t('architecture.eyebrow')}
            title={t('architecture.title')}
            lead={t('architecture.lead')}
          />

          <div className="mt-12 grid gap-12 lg:grid-cols-2 lg:items-start">
            <div>
              <ol className="divide-y divide-border/60 border-y border-border/60">
                {PIPELINE.map((stage) => (
                  <li key={stage.step} className="flex gap-5 py-5">
                    <span className="pt-0.5 font-mono text-xs text-peach">
                      {stage.step}
                    </span>
                    <div>
                      <h3 className="text-sm font-medium">
                        {t(`architecture.pipeline.${stage.key}.title`)}
                      </h3>
                      <p className="mt-1 text-sm leading-relaxed text-muted-foreground">
                        {t(`architecture.pipeline.${stage.key}.body`)}
                      </p>
                    </div>
                  </li>
                ))}
              </ol>
              <a
                href={site.links.architecture}
                target="_blank"
                rel="noreferrer"
                className="mt-8 inline-flex items-center gap-1 text-sm text-foreground/90 transition-colors hover:text-peach"
              >
                {t('architecture.docsLink')}
                <span
                  className="i-lucide-arrow-up-right size-3.5"
                  aria-hidden
                />
              </a>
            </div>

            <div>
              <VirtualListDemo />
              <p className="mt-3 text-center font-mono text-[11px] text-muted-foreground/60">
                {t('architecture.demoCaption')}
              </p>

              <h3 className="mt-10 text-sm font-medium">
                {t('architecture.gatesTitle')}
              </h3>
              <p className="mt-2 text-sm leading-relaxed text-muted-foreground">
                {t('architecture.gatesBody')}
              </p>
              <dl className="mt-5 divide-y divide-border/60 border-y border-border/60">
                {GATE_NUMBERS.map((row) => (
                  <div
                    key={row.key}
                    className="flex items-center justify-between py-2.5"
                  >
                    <dt className="text-sm text-muted-foreground">
                      {t(`architecture.gates.${row.key}`)}
                    </dt>
                    <dd className="font-mono text-sm tabular-nums">
                      {row.value}
                    </dd>
                  </div>
                ))}
              </dl>
              <a
                href={`${site.links.github}/tree/main/scripts`}
                target="_blank"
                rel="noreferrer"
                className="mt-6 inline-flex items-center gap-1 text-sm text-foreground/90 transition-colors hover:text-peach"
              >
                {t('architecture.gatesLink')}
                <span
                  className="i-lucide-arrow-up-right size-3.5"
                  aria-hidden
                />
              </a>
            </div>
          </div>
        </div>
      </Container>
    </section>
  )
}
