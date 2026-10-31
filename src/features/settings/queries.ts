import { type QueryClient, useMutation, useQuery } from '@tanstack/react-query'
import { ipc } from '../../lib/ipc'
import { isIpcError } from '../../lib/ipc/ipc-error'
import type { Settings, SettingsUpdate } from '../../lib/ipc/types'
import { queryKeys } from '../../shared/query-keys'
import { clampUiScale } from '../../shared/ui-scale'
import { appearanceUpdate, applySettingsUpdate } from './settings-update'
import type { OptimisticSettings } from './types'

// One scope runs settings writes one after another, so quick scale steps land in order.
const SETTINGS_SCOPE = { id: 'settings' }

async function applyOptimistically(
  client: QueryClient,
  update: SettingsUpdate,
): Promise<OptimisticSettings> {
  await client.cancelQueries({ queryKey: queryKeys.settings })
  const previous = client.getQueryData<Settings>(queryKeys.settings)
  if (previous) client.setQueryData(queryKeys.settings, applySettingsUpdate(previous, update))
  return { previous }
}

function rollBack(client: QueryClient, optimistic: OptimisticSettings | undefined): void {
  if (optimistic?.previous) client.setQueryData(queryKeys.settings, optimistic.previous)
}

// settings.json can turn read-only while RemMods runs (a newer version wrote it). The rollback
// snapshot still says writable, so the settings are fetched again to show the alert.
function refetchIfReadOnly(client: QueryClient, error: Error): void {
  if (isIpcError(error) && error.appError.kind === 'readOnly') {
    void client.invalidateQueries({ queryKey: queryKeys.settings })
  }
}

export function usePlatformInfo() {
  return useQuery({ queryKey: queryKeys.platformInfo, queryFn: () => ipc.platformInfo() })
}

export function useSettings() {
  return useQuery({ queryKey: queryKeys.settings, queryFn: () => ipc.settingsGet() })
}

export function useUpdateSettings() {
  return useMutation({
    scope: SETTINGS_SCOPE,
    mutationFn: (update: SettingsUpdate) => ipc.settingsUpdate(update),
    onMutate: (update, { client }) => applyOptimistically(client, update),
    onError: (error, _update, optimistic, { client }) => {
      rollBack(client, optimistic)
      refetchIfReadOnly(client, error)
    },
    onSuccess: (settings, _update, _optimistic, { client }) => {
      client.setQueryData(queryKeys.settings, settings)
    },
  })
}

// Saved first, zoomed second: if settings.json refuses the value, the zoom never diverges from it.
export function useSetUiScale() {
  return useMutation({
    scope: SETTINGS_SCOPE,
    mutationFn: async (scale: number) => {
      const settings = await ipc.settingsUpdate(appearanceUpdate({ uiScale: clampUiScale(scale) }))
      await ipc.setUiScale(settings.appearance.uiScale)
      return settings
    },
    onMutate: (scale, { client }) =>
      applyOptimistically(client, appearanceUpdate({ uiScale: clampUiScale(scale) })),
    onError: (error, _scale, optimistic, { client }) => {
      rollBack(client, optimistic)
      refetchIfReadOnly(client, error)
    },
    onSuccess: (settings, _scale, _optimistic, { client }) => {
      client.setQueryData(queryKeys.settings, settings)
    },
  })
}

export function useOpenLogs() {
  return useMutation({ mutationFn: () => ipc.openLogs() })
}
