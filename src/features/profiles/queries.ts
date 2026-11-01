import { useQuery } from '@tanstack/react-query'
import { ipc } from '../../lib/ipc'
import { queryKeys } from '../../shared/query-keys'

export function useProfiles() {
  return useQuery({ queryKey: queryKeys.profiles, queryFn: () => ipc.profilesList() })
}

// Never cached: a diff is only valid for the disk it was computed from.
export function useProfileDiff(profileId: string) {
  return useQuery({
    queryKey: queryKeys.profileDiff(profileId),
    queryFn: () => ipc.profilesDiff(profileId),
    gcTime: 0,
    staleTime: 0,
  })
}
