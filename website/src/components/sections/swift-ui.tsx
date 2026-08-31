import { useTranslation } from 'react-i18next'
import type { HighlightedSnippets } from '../../lib/snippets'
import { CodeBlock } from '../code-block'

export function SwiftUi({
  highlighted,
}: {
  highlighted: HighlightedSnippets
}) {
  const { t } = useTranslation()

  return (
    <section id="swift-ui" className="border-b border-border">
      <div className="grid gap-px bg-border lg:grid-cols-[2fr_3fr]">
        <div className="bg-background px-6 py-16 sm:px-12 sm:py-20">
          <div className="flex items-center gap-2 font-mono text-xs text-peach">
            <span className="i-simple-icons-swift size-4" aria-hidden />
            Native SwiftUI · macOS
          </div>
          <h2 className="mt-6 text-3xl font-semibold tracking-tight text-balance sm:text-4xl">
            {t('swiftUi.title')}
          </h2>
          <p className="mt-4 max-w-lg text-base leading-relaxed text-muted-foreground">
            {t('swiftUi.lead')}
          </p>
        </div>

        <div className="min-w-0 bg-card-2 p-6 sm:p-12">
          <div className="overflow-hidden border border-border bg-background">
            <div className="flex items-center justify-between border-b border-border bg-card-2 px-5 py-2.5">
              <span className="font-mono text-xs">swift-ui.tsx</span>
              <span className="flex items-center gap-1.5 font-mono text-[11px] text-muted-foreground">
                <span
                  className="i-simple-icons-solid size-3.5"
                  aria-hidden
                />
                TypeScript · Solid
              </span>
            </div>
            <CodeBlock html={highlighted.swiftUi} />
          </div>
        </div>
      </div>
    </section>
  )
}
