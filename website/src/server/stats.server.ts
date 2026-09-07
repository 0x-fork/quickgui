import type { RepoStats } from '../lib/stats'

const TTL_MS = 10 * 60_000

let cache: { value: RepoStats; at: number } | null = null

async function fetchJson(url: string): Promise<unknown> {
  const response = await fetch(url, {
    headers: {
      'user-agent': 'quickgui-website (https://github.com/egoist/quickgui)',
      accept: 'application/json',
    },
    signal: AbortSignal.timeout(2500),
  })
  if (!response.ok) throw new Error(`${url} responded ${response.status}`)
  return response.json()
}

export async function fetchRepoStats(): Promise<RepoStats> {
  if (cache && Date.now() - cache.at < TTL_MS) return cache.value

  const [github] = await Promise.allSettled([
    fetchJson('https://api.github.com/repos/egoist/quickgui'),
  ])

  const stars =
    github.status === 'fulfilled' &&
    typeof (github.value as { stargazers_count?: unknown })
      ?.stargazers_count === 'number'
      ? ((github.value as { stargazers_count: number }).stargazers_count)
      : null

  const value: RepoStats = { stars }
  if (stars !== null) {
    cache = { value, at: Date.now() }
  }
  return value
}
