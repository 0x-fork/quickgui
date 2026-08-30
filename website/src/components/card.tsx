import { cn } from '@/lib/utils'

export function Card({
  className,
  children,
}: {
  className?: string
  children: React.ReactNode
}) {
  return (
    <div
      className={cn(
        'rounded-2xl border border-border/60 bg-card',
        className,
      )}
    >
      {children}
    </div>
  )
}
