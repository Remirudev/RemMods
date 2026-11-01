import { notifications } from '@mantine/notifications'
import { type QueryClient, useMutation, useQueryClient } from '@tanstack/react-query'
import type { TFunction } from 'i18next'
import { useTranslation } from 'react-i18next'
import { ipc } from '../../lib/ipc'
import type { ModsSnapshot, ToggleOutcome, ToggleRequest } from '../../lib/ipc/types'
import { FS_SCOPE, routeFsError } from '../../shared/fs-error-route'
import { storeModsSnapshot } from '../../shared/mods-cache'
import { queryKeys } from '../../shared/query-keys'
import { disabledBetween, optimisticSnapshot } from './toggle-model'

interface PreviousSnapshot {
  previous: ModsSnapshot | undefined
}

async function takeSnapshot(client: QueryClient): Promise<ModsSnapshot | undefined> {
  await client.cancelQueries({ queryKey: queryKeys.mods })
  return client.getQueryData<ModsSnapshot>(queryKeys.mods)
}

function restoreSnapshot(client: QueryClient, context: PreviousSnapshot | undefined): void {
  if (context?.previous) client.setQueryData(queryKeys.mods, context.previous)
}

export function useToggleMods() {
  const client = useQueryClient()
  const mutation = useMutation<ToggleOutcome, unknown, ToggleRequest, PreviousSnapshot>({
    scope: FS_SCOPE,
    meta: { suppressErrorToast: true },
    mutationFn: (request) => ipc.modsToggle(request),
    onMutate: async (request) => {
      const previous = await takeSnapshot(client)
      if (previous) {
        const refs = request.targets.map((target) => target.modRef)
        client.setQueryData(queryKeys.mods, optimisticSnapshot(previous, refs, request.enable))
      }
      return { previous }
    },
    onSuccess: (outcome) => storeModsSnapshot(client, outcome.snapshot),
    onError: (error, request, context) => {
      restoreSnapshot(client, context)
      routeFsError(error, (applyWhileRunning) => {
        mutation.mutate({ ...request, applyWhileRunning })
      })
    },
  })
  return mutation
}

function disableAllToast(t: TFunction<'mods'>, count: number, outcome: ToggleOutcome): void {
  const lines: string[] = [t(($) => $.disableAll.done, { count })]
  if (outcome.skipped.length > 0) {
    lines.push(t(($) => $.disableAll.skipped, { count: outcome.skipped.length }))
  }
  if (outcome.unmanaged > 0) {
    lines.push(t(($) => $.disableAll.unmanaged, { count: outcome.unmanaged }))
  }
  notifications.show({ message: lines.join(' · '), autoClose: 6000 })
}

export function useDisableAllMods(afterSuccess?: () => void) {
  const client = useQueryClient()
  const { t } = useTranslation('mods')
  const mutation = useMutation<ToggleOutcome, unknown, boolean, PreviousSnapshot>({
    scope: FS_SCOPE,
    meta: { suppressErrorToast: true },
    mutationFn: (applyWhileRunning) => ipc.modsDisableAll(applyWhileRunning),
    onMutate: async () => ({ previous: await takeSnapshot(client) }),
    onSuccess: (outcome, _variables, context) => {
      storeModsSnapshot(client, outcome.snapshot)
      const count = context.previous ? disabledBetween(context.previous, outcome.snapshot) : 0
      disableAllToast(t, count, outcome)
      afterSuccess?.()
    },
    onError: (error) => {
      routeFsError(error, (applyWhileRunning) => mutation.mutate(applyWhileRunning))
    },
  })
  return mutation
}
