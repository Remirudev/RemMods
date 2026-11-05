import { useQuery } from '@tanstack/react-query'
import { ipc } from '../lib/ipc'
import { queryKeys } from './query-keys'

// Shared: the skin sets feature lists them, the group dialog links one.
export function useSkinSets() {
  return useQuery({ queryKey: queryKeys.skins, queryFn: () => ipc.skinsView() })
}
