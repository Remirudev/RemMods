import { getCurrentWindow } from '@tauri-apps/api/window'
import { describeForLog } from '../../lib/ipc/ipc-error'
import { logError } from '../../lib/log/frontend-log'
import type { AppWindow } from './types'

// getCurrentWindow() throws outside Tauri (a plain browser during UI work), so callers get null.
export function currentWindowOrNull(): AppWindow | null {
  try {
    return getCurrentWindow()
  } catch {
    return null
  }
}

export function runWindowAction(action: (appWindow: AppWindow) => Promise<void>): void {
  const appWindow = currentWindowOrNull()
  if (!appWindow) return
  action(appWindow).catch((error: unknown) => {
    logError(`window action failed: ${describeForLog(error)}`)
  })
}
