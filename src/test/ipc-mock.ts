import { mockIPC } from '@tauri-apps/api/mocks'
import { applySettingsUpdate } from '../features/settings/settings-update'
import type { Settings, SettingsUpdate } from '../lib/ipc/types'
import { TEST_PLATFORM_INFO, TEST_SETTINGS } from './fixtures'
import type { IpcCall, IpcMock } from './types'

// A stateful stand-in for the Rust commands; every call is recorded for assertions.
export function installIpcMock(initial: Settings = TEST_SETTINGS): IpcMock {
  const calls: IpcCall[] = []
  let settings = initial

  mockIPC((command, payload) => {
    calls.push({ command, payload })
    switch (command) {
      case 'settings_get':
        return settings
      case 'settings_update': {
        const { update } = payload as { update: SettingsUpdate }
        settings = applySettingsUpdate(settings, update)
        return settings
      }
      case 'platform_info':
        return TEST_PLATFORM_INFO
      default:
        return null
    }
  })

  return { calls }
}
