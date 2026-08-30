import { queryOptions } from '@tanstack/react-query'
import { fetchRepoStats } from '../server/stats'

export const repoStatsQueryOptions = () =>
  queryOptions({
    queryKey: ['repo-stats'],
    queryFn: () => fetchRepoStats(),
    staleTime: 10 * 60_000,
  })
