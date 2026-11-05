import { notifications } from '@mantine/notifications'
import { type QueryClient, useMutation, useQueryClient } from '@tanstack/react-query'
import type { TFunction } from 'i18next'
import { useTranslation } from 'react-i18next'
import { ipc } from '../../lib/ipc'
import type {
  ApplyRequest,
  ProfileSave,
  ProfilesView,
  TransferOutcome,
  TransferRequest,
} from '../../lib/ipc/types'
import { FS_SCOPE } from '../../shared/fs-error-route'
import { storeModsSnapshot } from '../../shared/mods-cache'
import { queryKeys } from '../../shared/query-keys'

function storeView(client: QueryClient, view: ProfilesView): void {
  client.setQueryData(queryKeys.profiles, view)
}

// The caller shows ProfileNameTaken and InvalidInput inline, so the global toast stays off.
export function useSaveProfile() {
  const client = useQueryClient()
  return useMutation({
    meta: { suppressErrorToast: true },
    mutationFn: (save: ProfileSave) => ipc.profilesSave(save),
    onSuccess: (view) => storeView(client, view),
  })
}

export function useDeleteProfile() {
  const client = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => ipc.profilesDelete(id),
    onSuccess: (view) => storeView(client, view),
  })
}

// Apply renames files, so it shares the scope of every other file change. Its failures and its
// toast (with Undo) are left to the caller: the diff modal, the sidebar switch or an undo.
export function useApplyProfile() {
  const client = useQueryClient()
  return useMutation({
    scope: FS_SCOPE,
    meta: { suppressErrorToast: true },
    mutationFn: (request: ApplyRequest) => ipc.profilesApply(request),
    onSuccess: (outcome) => {
      storeModsSnapshot(client, outcome.snapshot)
      void client.invalidateQueries({ queryKey: queryKeys.profiles })
      if (outcome.skins) void client.invalidateQueries({ queryKey: queryKeys.skins })
    },
  })
}

function transferToast(t: TFunction<'profiles'>, outcome: TransferOutcome) {
  if (outcome.kind === 'exported') {
    return t(($) => $.transfer.exported, {
      count: outcome.report.count,
      file: outcome.report.fileName,
    })
  }
  const { added, updated, unchanged, skipped } = outcome.report
  return t(($) => $.transfer.imported, { added, updated, unchanged, skipped })
}

export function useTransferProfiles() {
  const client = useQueryClient()
  const { t } = useTranslation('profiles')
  return useMutation({
    mutationFn: (request: TransferRequest) => ipc.profilesTransfer(request),
    onSuccess: (outcome) => {
      // null: the file dialog was cancelled.
      if (outcome === null) return
      if (outcome.kind === 'imported') storeView(client, outcome.report.view)
      notifications.show({ message: transferToast(t, outcome), autoClose: 8000 })
    },
  })
}
