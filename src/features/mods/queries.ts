import { useMutation, useQuery } from '@tanstack/react-query'
import { ipc } from '../../lib/ipc'
import { storeModsSnapshot } from '../../shared/mods-cache'
import { queryKeys } from '../../shared/query-keys'

export function useModsSnapshot(enabled: boolean) {
  return useQuery({ queryKey: queryKeys.mods, queryFn: () => ipc.modsList(), enabled })
}

// mods_list answers from the backend's cached scan; this rescans the disk.
export function useRefreshMods() {
  return useMutation({
    mutationFn: () => ipc.modsRefresh(),
    onSuccess: (snapshot, _variables, _onMutateResult, { client }) => {
      storeModsSnapshot(client, snapshot)
    },
  })
}

export function useWatchStatus() {
  return useQuery({ queryKey: queryKeys.watchStatus, queryFn: () => ipc.modsWatchStatus() })
}

export function useModDetails(modRef: string | null) {
  return useQuery({
    queryKey: queryKeys.modDetails(modRef ?? ''),
    queryFn: () => ipc.modsDetails(modRef ?? ''),
    enabled: modRef !== null,
  })
}

export function useRevealEntry() {
  return useMutation({ mutationFn: (modRef: string) => ipc.modsReveal(modRef) })
}
