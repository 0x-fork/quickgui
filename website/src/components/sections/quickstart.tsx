import { Trans, useTranslation } from 'react-i18next'
import { Card } from '../card'
import { Container } from '../container'
import { CodeBlock } from '../code-block'
import { SectionHeading } from '../section-heading'
import { site } from '../../lib/site'
import type { HighlightedSnippets } from '../../server/highlight'

function Step({
  number,
  title,
  children,
}: {
  number: string
  title: string
  children: React.ReactNode
}) {
  return (
    <div>
      <div className="flex items-baseline gap-3">
        <span className="font-mono text-xs text-peach">{number}</span>
        <h4 className="text-sm font-medium">{title}</h4>
      </div>
      <div className="mt-3 pl-7">{children}</div>
    </div>
  )
}

function Shell({ html }: { html: string }) {
  return (
    <div className="overflow-hidden rounded-lg border border-white/10 bg-[#101010]">
      <CodeBlock html={html} />
    </div>
  )
}

const mono = <span className="font-mono text-[13px]" />

export function Quickstart({
  highlighted,
}: {
  highlighted: HighlightedSnippets
}) {
  const { t } = useTranslation()

  return (
    <section id="quickstart">
      <Container>
        <SectionHeading
          index="04"
          eyebrow={t('quickstart.eyebrow')}
          title={t('quickstart.title')}
          lead={t('quickstart.lead')}
        />

        <div className="mt-10 grid gap-3 lg:grid-cols-2">
          <Card className="space-y-8 p-8 sm:p-10">
            <div className="flex items-center gap-3">
              <span
                className="i-simple-icons-rust size-5 text-foreground"
                aria-hidden
              />
              <h3 className="text-lg font-semibold tracking-tight">Rust</h3>
            </div>

            <Step number="01" title={t('quickstart.rust.addCrate')}>
              <Shell html={highlighted.cargoAdd} />
            </Step>
            <Step number="02" title={t('quickstart.rust.write')}>
              <p className="text-sm leading-relaxed text-muted-foreground">
                <Trans
                  i18nKey="quickstart.rust.writeBody"
                  components={{
                    lnk: (
                      <a
                        href="#code"
                        className="text-foreground underline decoration-border underline-offset-4 transition-colors hover:text-peach"
                      />
                    ),
                    c: mono,
                  }}
                />
              </p>
            </Step>
            <Step number="03" title={t('quickstart.rust.run')}>
              <Shell html={highlighted.cargoRun} />
            </Step>

            <p className="font-mono text-xs leading-relaxed text-muted-foreground/60">
              <Trans
                i18nKey="quickstart.rust.footnote"
                components={{
                  lnk: (
                    <a
                      href={site.links.docsRs}
                      target="_blank"
                      rel="noreferrer"
                      className="text-muted-foreground underline decoration-border underline-offset-4 hover:text-foreground"
                    />
                  ),
                }}
              />
            </p>
          </Card>

          <Card className="space-y-8 p-8 sm:p-10">
            <div className="flex items-center gap-3">
              <span
                className="i-simple-icons-solid size-5 text-foreground"
                aria-hidden
              />
              <h3 className="text-lg font-semibold tracking-tight">
                TypeScript · Solid 2
              </h3>
            </div>

            <Step number="01" title={t('quickstart.solid.scaffold')}>
              <Shell html={highlighted.cliInit} />
            </Step>
            <Step number="02" title={t('quickstart.solid.ship')}>
              <Shell html={highlighted.cliBuild} />
            </Step>

            <p className="text-sm leading-relaxed text-muted-foreground">
              <Trans i18nKey="quickstart.solid.body" components={{ c: mono }} />
            </p>

            <p className="font-mono text-xs leading-relaxed text-muted-foreground/60">
              <Trans
                i18nKey="quickstart.solid.footnote"
                components={{
                  lnk: (
                    <a
                      href={site.links.cli}
                      target="_blank"
                      rel="noreferrer"
                      className="text-muted-foreground underline decoration-border underline-offset-4 hover:text-foreground"
                    />
                  ),
                }}
              />
            </p>
          </Card>
        </div>
      </Container>
    </section>
  )
}
