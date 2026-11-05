import type { Channel } from '@tauri-apps/api/core'
import type {
  ApplyDirection,
  ApplyRequest,
  BulkDeleteRequest,
  BulkTrashRequest,
  DeleteRequest,
  GameStatus,
  InspectRequest,
  InstallProgress,
  LocationChoice,
  MetadataRequest,
  ModTarget,
  ProfileSave,
  RenameRequest,
  Settings,
  SettingsUpdate,
  StartRequest,
  ToggleRequest,
  TrashRequest,
} from '../../src/lib/ipc/types'
import {
  makeGraphicsState,
  TEST_INSTALL,
  TEST_NOTICES,
  TEST_UPDATE_CHANNEL,
} from '../../src/test/fixtures'
import { deleteMany, deletePrepareMany, trashMany } from './bulk'
import { inspect, releaseInstall, start } from './install-flow'
import * as mods from './mods'
import { applyProfile, deleteProfile, diffProfile, profilesView, saveProfile } from './profiles'
import { DETECTED, fail, snapshotOf } from './state'
import type { Backend, CommandArgs, E2eHandle, EventEmitter, MockState } from './types'

function arg<T>(args: CommandArgs, name: string): T {
  return args[name] as T
}

function applySettings(settings: Settings, update: SettingsUpdate): Settings {
  const patch = update.appearance
  if (!patch) return settings
  return {
    ...settings,
    appearance: {
      scheme: patch.scheme ?? settings.appearance.scheme,
      uiScale: patch.uiScale ?? settings.appearance.uiScale,
      titleBar: patch.titleBar ?? settings.appearance.titleBar,
    },
  }
}

function setLocation(state: MockState, choice: LocationChoice): GameStatus {
  if (choice.kind !== 'candidate' || choice.id !== DETECTED.id) {
    fail({ kind: 'invalidGameFolder', reason: 'notFound' })
  }
  state.game = {
    ...state.game,
    location: { kind: 'ready', install: { ...TEST_INSTALL } },
    candidates: [{ ...DETECTED, inUse: true }],
  }
  return state.game
}

// Window and log plugin calls only need a harmless answer; events are routed by mockIPC itself.
function pluginAnswer(cmd: string): unknown {
  if (cmd === 'plugin:window|is_maximized') return false
  return null
}

// One transparent pixel: enough for the page to show an <img>.
const TINY_PICTURE =
  'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg=='

function settingsCommand(state: MockState, cmd: string, args: CommandArgs): unknown {
  switch (cmd) {
    case 'platform_info':
      return state.platform
    case 'settings_get':
      return state.settings
    case 'settings_update':
      state.settings = applySettings(state.settings, arg<SettingsUpdate>(args, 'update'))
      return state.settings
    case 'update_channel':
      return TEST_UPDATE_CHANNEL
    case 'update_check':
      return { kind: 'upToDate' }
    case 'update_install':
      return fail({ kind: 'updateNotConfigured' })
    case 'graphics_get':
    case 'graphics_set_mode':
    case 'graphics_ack_step':
      return makeGraphicsState({ supported: state.platform.os === 'linux' })
    case 'assets_status':
    case 'assets_refresh':
      return { pictures: 12, bytes: 480_000, names: 120, failed: 0 }
    case 'assets_picture':
      return TINY_PICTURE
    case 'pick_picture':
      return { id: 'src-picture', name: 'look.png', kind: 'image' }
    case 'about_notices':
      return TEST_NOTICES
    case 'set_ui_scale':
    case 'app_ready':
    case 'open_logs':
    case 'smoke_report':
      return null
    default:
      return undefined
  }
}

function gameCommand(state: MockState, cmd: string, args: CommandArgs): unknown {
  switch (cmd) {
    case 'game_status':
    case 'game_ack_update':
      return state.game
    case 'game_detect':
      if (state.game.location.kind !== 'ready')
        state.game = { ...state.game, candidates: [DETECTED] }
      return state.game
    case 'game_set_location':
      return setLocation(state, arg<LocationChoice>(args, 'choice'))
    case 'game_pick_location':
      return { kind: 'cancelled' }
    case 'game_launch':
    case 'game_reveal':
      return null
    default:
      return undefined
  }
}

function modsCommand(
  state: MockState,
  cmd: string,
  args: CommandArgs,
  emit: EventEmitter,
): unknown {
  switch (cmd) {
    case 'mods_list':
    case 'mods_refresh':
      return snapshotOf(state)
    case 'mods_details':
      return mods.details(state, arg<string>(args, 'modRef'))
    case 'mods_contents':
      return mods.contents(state, arg<string>(args, 'modRef'))
    case 'mods_items':
      return mods.items(state, arg<string>(args, 'modRef'))
    case 'mods_set_picture':
      return mods.setPicture(state, arg<ModTarget>(args, 'target'))
    case 'mods_clear_picture':
      return mods.clearPicture(state, arg<ModTarget>(args, 'target'))
    case 'mods_reveal':
      return null
    case 'mods_watch_status':
      return { state: 'active' }
    case 'mods_toggle':
      return mods.toggle(state, arg<ToggleRequest>(args, 'request'), emit)
    case 'mods_disable_all':
      return mods.disableAll(state)
    case 'mods_rename':
      return mods.rename(state, arg<RenameRequest>(args, 'request'))
    case 'mods_set_metadata':
      return mods.setMetadata(state, arg<MetadataRequest>(args, 'request'))
    case 'mods_trash':
      return mods.trash(state, arg<TrashRequest>(args, 'request'))
    case 'mods_delete_prepare':
      return mods.deletePrepare(state, arg<ModTarget>(args, 'target'))
    case 'mods_delete_permanently':
      return mods.deletePermanently(state, arg<DeleteRequest>(args, 'request'))
    case 'mods_trash_many':
      return trashMany(state, arg<BulkTrashRequest>(args, 'request'))
    case 'mods_delete_prepare_many':
      return deletePrepareMany(state, arg<ModTarget[]>(args, 'targets'))
    case 'mods_delete_many':
      return deleteMany(state, arg<BulkDeleteRequest>(args, 'request'))
    case 'mods_busy_cancel':
      return mods.busyCancel(arg<number>(args, 'waitId'))
    case 'repair_list':
      return { revision: state.revision, items: [] }
    default:
      return undefined
  }
}

function profilesCommand(state: MockState, cmd: string, args: CommandArgs): unknown {
  switch (cmd) {
    case 'profiles_list':
      return profilesView(state)
    case 'profiles_save':
      return saveProfile(state, arg<ProfileSave>(args, 'save'))
    case 'profiles_delete':
      return deleteProfile(state, arg<string>(args, 'id'))
    case 'profiles_diff':
      return diffProfile(state, arg<string>(args, 'id'), arg<ApplyDirection>(args, 'direction'))
    case 'profiles_apply':
      return applyProfile(state, arg<ApplyRequest>(args, 'request'))
    case 'profiles_transfer':
      return null
    default:
      return undefined
  }
}

function installCommand(state: MockState, cmd: string, args: CommandArgs): unknown {
  switch (cmd) {
    case 'pick_archives':
      return []
    case 'pick_folder':
      return null
    case 'install_inspect':
      return inspect(state, arg<InspectRequest>(args, 'request'))
    case 'install_start':
      return start(
        state,
        arg<StartRequest>(args, 'request'),
        arg<Channel<InstallProgress>>(args, 'progress'),
      )
    case 'install_cancel':
      return false
    default:
      return undefined
  }
}

// JSON copies keep the recorded payloads readable through page.evaluate; a Channel becomes its
// `__CHANNEL__:<id>` form.
function recordable(args: CommandArgs): unknown {
  return JSON.parse(JSON.stringify(args ?? {}))
}

export function createBackend(state: MockState, emit: EventEmitter): Backend {
  const handle: E2eHandle = { calls: [], state, unknown: [], emit, releaseInstall }

  const answer = (cmd: string, args: CommandArgs): unknown => {
    for (const group of [settingsCommand, gameCommand, profilesCommand, installCommand]) {
      const result = group(state, cmd, args)
      if (result !== undefined) return result
    }
    const result = modsCommand(state, cmd, args, emit)
    if (result !== undefined) return result
    handle.unknown.push(cmd)
    return fail({ kind: 'internal' })
  }

  // Answers are copies, so the app never shares an object with the mock state.
  const route = (cmd: string, args: CommandArgs): unknown => {
    if (cmd.startsWith('plugin:')) return pluginAnswer(cmd)
    handle.calls.push({ cmd, args: recordable(args) })
    const result = answer(cmd, args ?? {})
    if (result instanceof Promise) return result.then((value) => structuredClone(value))
    return structuredClone(result)
  }

  return { handle, route }
}
