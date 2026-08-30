import { cn } from '@/lib/utils'

export function WindowFrame({
  title,
  className,
  children,
}: {
  title: string
  className?: string
  children: React.ReactNode
}) {
  return (
    <div
      className={cn(
        'overflow-hidden rounded-xl border border-white/10 bg-[#121214] shadow-[0_30px_90px_-30px_rgba(0,0,0,0.9),0_0_0_1px_rgba(255,255,255,0.02)]',
        className,
      )}
    >
      <div className="relative flex h-9 shrink-0 items-center border-b border-white/[0.06] px-3.5">
        <div className="flex gap-2" aria-hidden>
          <span className="size-3 rounded-full bg-[#ff5f57]" />
          <span className="size-3 rounded-full bg-[#febc2e]" />
          <span className="size-3 rounded-full bg-[#28c840]" />
        </div>
        <span className="pointer-events-none absolute inset-x-0 text-center font-mono text-[11px] text-white/35">
          {title}
        </span>
      </div>
      {children}
    </div>
  )
}
