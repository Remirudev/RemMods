import { mockIPC } from '@tauri-apps/api/mocks'
import { applySettingsUpdate } from '../features/settings/settings-update'
import type { Settings, SettingsUpdate } from '../lib/ipc/types'
import {
  makeGameStatus,
  makeGraphicsState,
  TEST_CONTENTS,
  TEST_DETAILS,
  TEST_NOTICES,
  TEST_PLATFORM_INFO,
  TEST_SETTINGS,
  TEST_SNAPSHOT,
  TEST_UPDATE_CHANNEL,
} from './fixtures'
import type { IpcCall, IpcMock, IpcMockOptions } from './types'

// A stateful stand-in for the Rust commands; every call is recorded for assertions. Game and mod
// commands answer from the options; `handlers` replaces or extends any command.
export function installIpcMock(
  initial: Settings = TEST_SETTINGS,
  options: IpcMockOptions = {},
): IpcMock {
  const calls: IpcCall[] = []
  let settings = initial
  const gameStatus = options.gameStatus ?? makeGameStatus()
  const snapshot = options.snapshot ?? TEST_SNAPSHOT
  const details = options.details ?? TEST_DETAILS
  const contents = options.contents ?? TEST_CONTENTS

  mockIPC(
    (command, payload) => {
      calls.push({ command, payload })
      const handler = options.handlers?.[command]
      if (handler) return handler(payload)
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
        case 'game_status':
        case 'game_detect':
        case 'game_ack_update':
        case 'game_set_location':
          return gameStatus
        case 'mods_list':
        case 'mods_refresh':
          return snapshot
        case 'mods_details':
          return details
        case 'mods_contents':
          return contents
        case 'mods_watch_status':
          return { state: 'active' }
        case 'update_channel':
          return TEST_UPDATE_CHANNEL
        case 'graphics_get':
          return makeGraphicsState()
        case 'about_notices':
          return TEST_NOTICES
        default:
          return null
      }
    },
    { shouldMockEvents: true },
  )

  return { calls, callsOf: (command) => calls.filter((call) => call.command === command) }
}
