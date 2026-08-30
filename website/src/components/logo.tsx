import { cn } from '@/lib/utils'

export function Logo({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 32 32"
      fill="none"
      aria-hidden
      className={cn('size-6', className)}
    >
      <rect
        x="7"
        y="12"
        width="13"
        height="13"
        rx="3"
        stroke="var(--peach)"
        strokeOpacity="0.5"
        strokeWidth="1.75"
      />
      <rect x="12" y="7" width="13" height="13" rx="3" fill="var(--peach)" />
    </svg>
  )
}
