import { useTranslation } from 'react-i18next'
import { Card } from '../card'
import { Container } from '../container'

const STATS = [
  { value: '0', key: 'stats.idle' },
  { value: '95 Hz', key: 'stats.scroll' },
  { value: '1.35 ms', key: 'stats.cpu' },
  { value: '561', key: 'stats.tests' },
] as const

export function StatsBand() {
  const { t } = useTranslation()

  return (
    <section>
      <Container>
        <Card className="overflow-hidden">
          <dl className="grid grid-cols-2 gap-px bg-border/60 lg:grid-cols-4">
            {STATS.map((stat) => (
              <div key={stat.key} className="bg-card p-8 sm:p-10">
                <dd className="font-mono text-3xl tracking-tight sm:text-5xl">
                  {stat.value}
                </dd>
                <dt className="mt-3 text-sm text-muted-foreground">
                  {t(stat.key)}
                </dt>
              </div>
            ))}
          </dl>
        </Card>
      </Container>
    </section>
  )
}
