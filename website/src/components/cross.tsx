import { cn } from '@/lib/utils'

/**
 * A small `+` marker centered on a border intersection. The parent must be
 * `relative`; position with `top-0 left-0`, `top-0 left-full`, etc. Callers
 * hide it on viewports where the column borders touch the screen edge
 * (`hidden xl:block`), otherwise the overhang causes horizontal overflow.
 */
export function Cross({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 12 12"
      aria-hidden
      className={cn(
        'pointer-events-none absolute z-10 size-[11px] -translate-x-1/2 -translate-y-1/2 text-foreground/35',
        className,
      )}
    >
      <path d="M6 0v12M0 6h12" stroke="currentColor" strokeWidth="1" />
    </svg>
  )
}
