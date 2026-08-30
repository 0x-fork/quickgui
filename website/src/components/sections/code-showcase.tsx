import { useTranslation } from 'react-i18next'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Container } from '../container'
import { CodeBlock } from '../code-block'
import { SectionHeading } from '../section-heading'
import { site } from '../../lib/site'
import type { HighlightedSnippets } from '../../server/highlight'

const POINTS = [
  { icon: 'i-lucide-braces', key: 'builders' },
  { icon: 'i-lucide-mouse-pointer-click', key: 'listeners' },
  { icon: 'i-lucide-paintbrush', key: 'paint' },
] as const

export function CodeShowcase({
  highlighted,
}: {
  highlighted: HighlightedSnippets
}) {
  const { t } = useTranslation()

  return (
    <section id="code">
      <Container>
        <div className="rounded-3xl border border-border/60 bg-card p-6 sm:p-12">
          <div className="grid gap-12 lg:grid-cols-[minmax(0,5fr)_minmax(0,7fr)] lg:items-start">
            <div>
              <SectionHeading
                index="02"
                eyebrow={t('code.eyebrow')}
                title={t('code.title')}
                lead={t('code.lead')}
              />

              <ul className="mt-10 space-y-6">
                {POINTS.map((point) => (
                  <li key={point.key} className="flex gap-4">
                    <span
                      className={`${point.icon} mt-0.5 size-4.5 shrink-0 text-peach`}
                      aria-hidden
                    />
                    <div>
                      <h3 className="text-sm font-medium">
                        {t(`code.points.${point.key}.title`)}
                      </h3>
                      <p className="mt-1 text-sm leading-relaxed text-muted-foreground">
                        {t(`code.points.${point.key}.body`)}
                      </p>
                    </div>
                  </li>
                ))}
              </ul>

              <div className="mt-10 flex flex-wrap gap-x-6 gap-y-2 text-sm">
                <a
                  href={site.links.viewApi}
                  target="_blank"
                  rel="noreferrer"
                  className="flex items-center gap-1 text-foreground/90 transition-colors hover:text-peach"
                >
                  {t('code.viewApiDocs')}
                  <span
                    className="i-lucide-arrow-up-right size-3.5"
                    aria-hidden
                  />
                </a>
                <a
                  href={site.links.solid}
                  target="_blank"
                  rel="noreferrer"
                  className="flex items-center gap-1 text-foreground/90 transition-colors hover:text-peach"
                >
                  {t('code.solidDocs')}
                  <span
                    className="i-lucide-arrow-up-right size-3.5"
                    aria-hidden
                  />
                </a>
              </div>
            </div>

            <Tabs defaultValue="rust" className="min-w-0">
              <div className="overflow-hidden rounded-xl border border-white/10 bg-[#101010] shadow-[0_30px_90px_-30px_rgba(0,0,0,0.9)]">
                <div className="flex items-center justify-between border-b border-white/[0.06] px-3 py-2">
                  <TabsList className="h-8 gap-1 bg-transparent p-0">
                    <TabsTrigger
                      value="rust"
                      className="h-7 rounded-md px-3 font-mono text-xs data-[state=active]:bg-white/[0.07] data-[state=active]:text-foreground"
                    >
                      main.rs
                    </TabsTrigger>
                    <TabsTrigger
                      value="solid"
                      className="h-7 rounded-md px-3 font-mono text-xs data-[state=active]:bg-white/[0.07] data-[state=active]:text-foreground"
                    >
                      app.tsx
                    </TabsTrigger>
                  </TabsList>
                  <span className="hidden font-mono text-[11px] text-white/30 sm:block">
                    theme: vesper
                  </span>
                </div>
                <TabsContent value="rust" className="mt-0">
                  <CodeBlock
                    html={highlighted.rust}
                    className="max-h-[560px] overflow-auto"
                  />
                </TabsContent>
                <TabsContent value="solid" className="mt-0">
                  <CodeBlock
                    html={highlighted.solid}
                    className="max-h-[560px] overflow-auto"
                  />
                  <p className="border-t border-white/[0.06] px-4 py-3 font-mono text-[11px] text-white/40">
                    Solid 2 on Bun via N-API · packaged by @quickgui/cli
                  </p>
                </TabsContent>
              </div>
            </Tabs>
          </div>
        </div>
      </Container>
    </section>
  )
}
