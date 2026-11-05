import { type QueryClient, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ipc } from '../../lib/ipc'
import type { SkinsView } from '../../lib/ipc/types'
import { FS_SCOPE } from '../../shared/fs-error-route'
import { queryKeys } from '../../shared/query-keys'
import type { NewSkinSet } from './types'

function storeView(client: QueryClient, view: SkinsView): void {
  client.setQueryData(queryKeys.skins, view)
}

export function useSaveNewSkinSet() {
  const client = useQueryClient()
  return useMutation({
    scope: FS_SCOPE,
    meta: { suppressErrorToast: true },
    mutationFn: ({ name, copy }: NewSkinSet) =>
      copy ? ipc.skinsKeepCopy(copy, name) : ipc.skinsSaveNew(name),
    onSuccess: (view) => storeView(client, view),
  })
}

function useSwapSaves(swap: (id: string) => Promise<SkinsView>, onDone?: () => void) {
  const client = useQueryClient()
  return useMutation({
    scope: FS_SCOPE,
    mutationFn: swap,
    onSuccess: (view) => {
      storeView(client, view)
      onDone?.()
    },
    // A failed switch can leave the list changed, so it is fetched again either way.
    onError: () => client.invalidateQueries({ queryKey: queryKeys.skins }),
  })
}

export function useSwitchSkinSet() {
  return useSwapSaves(ipc.skinsSwitch)
}

// `onDone` is on the hook because a callback on `mutate` would be lost: the confirm modal takes
// the place of the dialog that asked, which unmounts it.
export function usePutBackSkinSet(onDone: () => void) {
  return useSwapSaves(ipc.skinsPutBack, onDone)
}

// Every switch adds copies, so the list is read fresh each time it opens.
export function useSkinCopies() {
  return useQuery({ queryKey: queryKeys.skinCopies, queryFn: ipc.skinsCopies, gcTime: 0 })
}

export function useEditSkinSet() {
  const client = useQueryClient()
  return useMutation({
    scope: FS_SCOPE,
    meta: { suppressErrorToast: true },
    mutationFn: ({ id, name, notes }: { id: string; name: string; notes: string }) =>
      ipc.skinsEdit(id, name, notes),
    onSuccess: (view) => storeView(client, view),
  })
}

// `onDone` is on the hook for the same reason as in `usePutBackSkinSet`.
export function useDeleteSkinSet(onDone: () => void) {
  const client = useQueryClient()
  return useMutation({
    scope: FS_SCOPE,
    mutationFn: (id: string) => ipc.skinsDelete(id),
    onSuccess: (view) => {
      storeView(client, view)
      onDone()
    },
  })
}

export function useAddScreenshots(id: string) {
  const client = useQueryClient()
  return useMutation({
    scope: FS_SCOPE,
    mutationFn: async (title: string) => {
      for (const source of await ipc.pickScreenshots(title)) {
        storeView(client, await ipc.skinsAddScreenshot(id, source.id))
      }
    },
  })
}

export function useRemoveScreenshot(id: string) {
  const client = useQueryClient()
  return useMutation({
    scope: FS_SCOPE,
    mutationFn: (shot: string) => ipc.skinsRemoveScreenshot(id, shot),
    onSuccess: (view) => storeView(client, view),
  })
}
