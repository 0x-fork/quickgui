import { cn } from '@/lib/utils'

export function CodeBlock({
  html,
  className,
}: {
  html: string
  className?: string
}) {
  return (
    <div
      className={cn('code-panel slim-scroll text-[13px]', className)}
      // Trusted HTML: produced by shiki on the server from our own snippets.
      dangerouslySetInnerHTML={{ __html: html }}
    />
  )
}
