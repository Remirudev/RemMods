import { useQuery } from '@tanstack/react-query'
import { ipc } from '../../lib/ipc'
import type { ApplyDirection } from '../../lib/ipc/types'
import { queryKeys } from '../../shared/query-keys'

export function useProfiles() {
  return useQuery({ queryKey: queryKeys.profiles, queryFn: () => ipc.profilesList() })
}

// Never cached: a diff is only valid for the disk it was computed from.
export function useProfileDiff(profileIds: readonly string[], direction: ApplyDirection) {
  return useQuery({
    queryKey: queryKeys.profileDiff(profileIds, direction),
    queryFn: () => ipc.profilesDiff([...profileIds], direction),
    gcTime: 0,
    staleTime: 0,
  })
}
