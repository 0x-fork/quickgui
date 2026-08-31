import { useTranslation } from 'react-i18next'
import { CodeBlock } from '../code-block'
import { SectionHeading } from '../section-heading'
import type { HighlightedSnippets } from '../../server/highlight'

function Step({
  number,
  title,
  children,
}: {
  number: string
  title: string
  children?: React.ReactNode
}) {
  return (
    <div>
      <div className="flex items-baseline gap-3">
        <span className="font-mono text-xs text-peach">{number}</span>
        <h4 className="text-[15px] font-medium">{title}</h4>
      </div>
      {children ? <div className="mt-4">{children}</div> : null}
    </div>
  )
}

function Shell({ html }: { html: string }) {
  return (
    <div className="overflow-hidden border border-border bg-card-2">
      <CodeBlock html={html} />
    </div>
  )
}

export function Quickstart({
  highlighted,
}: {
  highlighted: HighlightedSnippets
}) {
  const { t } = useTranslation()

  return (
    <section id="quickstart" className="border-b border-border">
      <SectionHeading title={t('quickstart.title')} />

      <div className="grid gap-px bg-border lg:grid-cols-2">
        <div className="space-y-9 bg-background p-8 sm:p-12">
          <div className="flex items-center gap-2.5">
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
            <p className="text-[15px] leading-relaxed text-muted-foreground">
              {t('quickstart.rust.writeBody')}
            </p>
          </Step>
          <Step number="03" title={t('quickstart.rust.run')}>
            <Shell html={highlighted.cargoRun} />
          </Step>
        </div>

        <div className="space-y-9 bg-background p-8 sm:p-12">
          <div className="flex items-center gap-2.5">
            <span
              className="i-simple-icons-solid size-5 text-foreground"
              aria-hidden
            />
            <h3 className="text-lg font-semibold tracking-tight">
              TypeScript · Solid
            </h3>
          </div>

          <Step number="01" title={t('quickstart.solid.create')}>
            <Shell html={highlighted.cliInit} />
          </Step>
          <Step number="02" title={t('quickstart.solid.ship')}>
            <Shell html={highlighted.cliBuild} />
          </Step>

          <p className="text-[15px] leading-relaxed text-muted-foreground">
            {t('quickstart.solid.note')}
          </p>
        </div>
      </div>
    </section>
  )
}
