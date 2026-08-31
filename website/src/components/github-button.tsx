import { site } from '../lib/site'
import type { RepoStats } from '../lib/stats'

function formatCount(value: number): string {
  if (value >= 1000) {
    return `${(value / 1000).toFixed(value >= 10_000 ? 0 : 1)}k`
  }
  return String(value)
}

export function GitHubButton({ stats }: { stats: RepoStats }) {
  return (
    <a
      href={site.links.github}
      target="_blank"
      rel="noreferrer"
      className="flex h-8 items-center gap-2 border border-border bg-background px-3 text-[13px] text-foreground/90 transition-colors hover:bg-muted"
    >
      <span className="i-simple-icons-github size-3.5" aria-hidden />
      <span className="hidden sm:inline">GitHub</span>
      {typeof stats.stars === 'number' ? (
        <span className="flex items-center gap-1 font-mono text-[11px] text-muted-foreground tabular-nums">
          <span className="i-lucide-star size-3" aria-hidden />
          {formatCount(stats.stars)}
        </span>
      ) : null}
    </a>
  )
}
