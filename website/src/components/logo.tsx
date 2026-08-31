import { cn } from '@/lib/utils'

/* Generated brand mark (public/logo.png): a Q whose tail is a lightning
   bolt. White background, so it sits on white surfaces only. */
export function Logo({ className }: { className?: string }) {
  return (
    <img
      src="/logo.png"
      alt=""
      aria-hidden
      className={cn('size-6', className)}
    />
  )
}
