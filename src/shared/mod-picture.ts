import type { QueryClient } from '@tanstack/react-query'
import { ipc } from '../lib/ipc'
import type { ModTarget } from '../lib/ipc/types'
import { queryKeys } from './query-keys'

// The one way an image becomes a mod's picture: picked in the details or dropped on the window.
export async function applyModPicture(
  client: QueryClient,
  target: ModTarget,
  sourceId: string,
): Promise<void> {
  await ipc.modsSetPicture(target, sourceId)
  await client.invalidateQueries({ queryKey: queryKeys.modItemsAll })
}
