import type { QueryClient } from '@tanstack/react-query'
import { ipc } from '../lib/ipc'
import { describeForLog } from '../lib/ipc/ipc-error'
import { logError } from '../lib/log/frontend-log'
import { queryKeys } from '../shared/query-keys'
import type { PreloadedAppData } from './types'

async function loadOrNull<T>(command: string, load: () => Promise<T>): Promise<T | null> {
  try {
    return await load()
  } catch (error) {
    logError(`${command} failed during startup: ${describeForLog(error)}`)
    return null
  }
}

// Loaded before the first render, so the first frame already has the saved colour scheme, the
// title bar the window was created with and the game folder (no "looking for TEKKEN 8" flash).
// A failure leaves the queries to retry on mount.
export async function preloadAppData(queryClient: QueryClient): Promise<PreloadedAppData> {
  const [settings, platformInfo, gameStatus] = await Promise.all([
    loadOrNull('settings_get', () => ipc.settingsGet()),
    loadOrNull('platform_info', () => ipc.platformInfo()),
    loadOrNull('game_status', () => ipc.gameStatus()),
  ])
  if (settings) queryClient.setQueryData(queryKeys.settings, settings)
  if (platformInfo) queryClient.setQueryData(queryKeys.platformInfo, platformInfo)
  if (gameStatus) queryClient.setQueryData(queryKeys.gameStatus, gameStatus)
  return { settings, platformInfo, gameStatus }
}
