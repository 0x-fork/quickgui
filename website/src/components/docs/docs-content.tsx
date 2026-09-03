import type { ReactNode } from 'react'

export function Note({
  title = 'NOTE',
  children,
  tone = 'neutral',
}: {
  title?: string
  children: ReactNode
  tone?: 'neutral' | 'accent'
}) {
  return (
    <aside className="docs-note" data-tone={tone}>
      <strong>{title}</strong>
      <div>{children}</div>
    </aside>
  )
}
