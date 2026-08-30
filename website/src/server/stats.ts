import { createServerFn } from '@tanstack/react-start'

export interface RepoStats {
  stars: number | null
  version: string | null
}

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

export const fetchRepoStats = createServerFn({ method: 'GET' }).handler(
  async (): Promise<RepoStats> => {
    if (cache && Date.now() - cache.at < TTL_MS) return cache.value

    const [github, crate] = await Promise.allSettled([
      fetchJson('https://api.github.com/repos/egoist/quickgui'),
      fetchJson('https://crates.io/api/v1/crates/quickgui'),
    ])

    const stars =
      github.status === 'fulfilled' &&
      typeof (github.value as { stargazers_count?: unknown })
        ?.stargazers_count === 'number'
        ? ((github.value as { stargazers_count: number }).stargazers_count)
        : null

    const crateData =
      crate.status === 'fulfilled'
        ? (crate.value as { crate?: { max_stable_version?: unknown } }).crate
        : undefined
    const version =
      typeof crateData?.max_stable_version === 'string'
        ? crateData.max_stable_version
        : null

    const value: RepoStats = { stars, version }
    if (stars !== null || version !== null) {
      cache = { value, at: Date.now() }
    }
    return value
  },
)
