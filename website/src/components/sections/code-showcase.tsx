import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import { CodeBlock } from '../code-block'
import { SectionHeading } from '../section-heading'
import type { HighlightedSnippets } from '../../lib/snippets'

const PANES = [
  {
    key: 'rust',
    file: 'main.rs',
    icon: 'i-simple-icons-rust',
    label: 'Rust',
  },
  {
    key: 'ui',
    file: 'app.tsx',
    icon: 'i-simple-icons',
    label: 'TypeScript · QuickGUI UI',
  },
] as const

export function CodeShowcase({
  highlighted,
}: {
  highlighted: HighlightedSnippets
}) {
  const { t } = useTranslation()
  const [active, setActive] = useState<(typeof PANES)[number]['key']>('rust')

  return (
    <section id="code" className="border-b border-border">
      <SectionHeading title={t('code.title')} lead={t('code.lead')} />

      {/* Mobile: tab bar switches the panes; desktop shows both side by side. */}
      <div className="flex border-b border-border lg:hidden">
        {PANES.map((pane) => (
          <button
            key={pane.key}
            type="button"
            aria-pressed={active === pane.key}
            onClick={() => setActive(pane.key)}
            className={cn(
              'flex-1 border-r border-border py-3 font-mono text-xs transition-colors last:border-r-0',
              active === pane.key
                ? 'bg-background text-foreground'
                : 'bg-card-2 text-muted-foreground',
            )}
          >
            {pane.file}
          </button>
        ))}
      </div>

      <div className="grid gap-px bg-border lg:grid-cols-2">
        {PANES.map((pane) => (
          <div
            key={pane.key}
            className={cn(
              'min-w-0 bg-background',
              active !== pane.key && 'hidden lg:block',
            )}
          >
            <div className="hidden items-center justify-between border-b border-border bg-card-2 px-5 py-2.5 lg:flex">
              <span className="font-mono text-xs">{pane.file}</span>
              <span className="flex items-center gap-1.5 font-mono text-[11px] text-muted-foreground">
                <span className={`${pane.icon} size-3.5`} aria-hidden />
                {pane.label}
              </span>
            </div>
            <CodeBlock html={highlighted[pane.key]} />
          </div>
        ))}
      </div>
    </section>
  )
}
