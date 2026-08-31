import { useTranslation } from 'react-i18next'

const PLATFORMS = [
  { name: 'macOS', available: true },
  { name: 'Windows', available: false },
  { name: 'Linux', available: false },
] as const

export function Platforms() {
  const { t } = useTranslation()

  return (
    <section className="border-b border-border">
      <div className="grid gap-px bg-border sm:grid-cols-3">
        {PLATFORMS.map((platform) => (
          <div
            key={platform.name}
            className="flex items-center gap-3 bg-background px-8 py-6 sm:px-12"
          >
            <span
              className={`size-1.5 rounded-full ${platform.available ? 'bg-mint' : 'bg-peach'}`}
              aria-hidden
            />
            <span className="text-[15px] font-medium">{platform.name}</span>
            <span
              className={`font-mono text-xs ${platform.available ? 'text-mint' : 'text-muted-foreground'}`}
            >
              {t(platform.available ? 'platforms.available' : 'platforms.soon')}
            </span>
          </div>
        ))}
      </div>
    </section>
  )
}
