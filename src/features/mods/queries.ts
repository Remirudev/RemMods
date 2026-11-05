import { useMutation, useQuery } from '@tanstack/react-query'
import { ipc } from '../../lib/ipc'
import type { ModTarget } from '../../lib/ipc/types'
import { applyModPicture } from '../../shared/mod-picture'
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

export function useModContents(modRef: string | null) {
  return useQuery({
    queryKey: queryKeys.modContents(modRef ?? ''),
    queryFn: () => ipc.modsContents(modRef ?? ''),
    enabled: modRef !== null,
  })
}

export function useModItems(modRef: string, language: string) {
  return useQuery({
    queryKey: queryKeys.modItems(modRef, language),
    queryFn: () => ipc.modsItems(modRef, language),
  })
}

// The dialog runs in Rust and hands back an id; cancelling it changes nothing.
export function usePickModPicture(target: ModTarget) {
  return useMutation({
    mutationFn: (title: string) => ipc.pickPicture(title),
    onSuccess: async (source, _title, _result, { client }) => {
      if (source) await applyModPicture(client, target, source.id)
    },
  })
}

export function useClearModPicture(target: ModTarget) {
  return useMutation({
    mutationFn: () => ipc.modsClearPicture(target),
    onSuccess: (_result, _variables, _onMutateResult, { client }) =>
      client.invalidateQueries({ queryKey: queryKeys.modItemsAll }),
  })
}

export function useRevealEntry() {
  return useMutation({ mutationFn: (modRef: string) => ipc.modsReveal(modRef) })
}

export function useRepairList() {
  return useQuery({ queryKey: queryKeys.repair, queryFn: () => ipc.repairList() })
}
