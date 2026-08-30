import { cn } from '@/lib/utils'

export function SectionHeading({
  index,
  eyebrow,
  title,
  lead,
  align = 'left',
  className,
}: {
  index?: string
  eyebrow: string
  title: string
  lead?: string
  align?: 'left' | 'center'
  className?: string
}) {
  return (
    <div
      className={cn(
        'max-w-2xl',
        align === 'center' && 'mx-auto text-center',
        className,
      )}
    >
      <p
        className={cn(
          'flex items-center gap-2.5 font-mono text-xs tracking-[0.22em] text-muted-foreground uppercase',
          align === 'center' && 'justify-center',
        )}
      >
        {index ? (
          <>
            <span className="text-peach">{index}</span>
            <span className="text-muted-foreground/40">/</span>
          </>
        ) : null}
        <span>{eyebrow}</span>
      </p>
      <h2 className="mt-4 text-3xl font-semibold tracking-tight text-balance sm:text-4xl">
        {title}
      </h2>
      {lead ? (
        <p className="mt-4 text-base leading-relaxed text-muted-foreground">
          {lead}
        </p>
      ) : null}
    </div>
  )
}
