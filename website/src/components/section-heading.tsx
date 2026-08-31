import { cn } from '@/lib/utils'

/** The intro strip at the top of a section; sits on its own hairline row. */
export function SectionHeading({
  title,
  lead,
  className,
}: {
  title: string
  lead?: string
  className?: string
}) {
  return (
    <div
      className={cn(
        'border-b border-border px-6 py-16 sm:px-12 sm:py-20',
        className,
      )}
    >
      <h2 className="max-w-2xl text-3xl font-semibold tracking-tight text-balance sm:text-4xl">
        {title}
      </h2>
      {lead ? (
        <p className="mt-4 max-w-2xl text-base leading-relaxed text-muted-foreground">
          {lead}
        </p>
      ) : null}
    </div>
  )
}
