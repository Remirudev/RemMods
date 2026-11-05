import type { QueryClient } from '@tanstack/react-query'
import type { ModsSnapshot } from '../lib/ipc/types'
import { queryKeys } from './query-keys'

// A new revision means the disk changed, so cached file lists of open details are refetched too.
export function storeModsSnapshot(client: QueryClient, snapshot: ModsSnapshot): void {
  const previous = client.getQueryData<ModsSnapshot>(queryKeys.mods)
  client.setQueryData(queryKeys.mods, snapshot)
  if (previous && previous.revision !== snapshot.revision) {
    void client.invalidateQueries({ queryKey: queryKeys.modDetailsAll })
    void client.invalidateQueries({ queryKey: queryKeys.repair })
    void client.invalidateQueries({ queryKey: queryKeys.profiles })
  }
}

// A bulk call whose final rescan failed reports no snapshot (its rows still count): the cached
// list is marked stale and fetched again instead of being overwritten.
export function storeOrRefetchMods(client: QueryClient, snapshot: ModsSnapshot | null): void {
  if (snapshot) {
    storeModsSnapshot(client, snapshot)
    return
  }
  void client.invalidateQueries({ queryKey: queryKeys.mods })
  void client.invalidateQueries({ queryKey: queryKeys.repair })
  void client.invalidateQueries({ queryKey: queryKeys.profiles })
}
