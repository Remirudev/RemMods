import { useQuery } from '@tanstack/react-query'
import { ipc } from '../lib/ipc'
import { queryKeys } from './query-keys'

// An id names the picture's content, so a loaded picture never changes.
export function useItemPicture(id: string | null) {
  return useQuery({
    queryKey: queryKeys.itemPicture(id ?? ''),
    queryFn: () => ipc.assetsPicture(id ?? ''),
    enabled: id !== null,
    staleTime: Number.POSITIVE_INFINITY,
  })
}
