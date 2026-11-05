import type { UnlistenFn } from '@tauri-apps/api/event'
import { events } from './bindings'
import type {
  AssetsChanged,
  FolderBusy,
  GameStatusChanged,
  InstallDrop,
  ModsChanged,
  WatchStatusChanged,
} from './types'

// The backend does not buffer events: subscribe first, then fetch what the event would update.
export function onModsChanged(callback: (payload: ModsChanged) => void): Promise<UnlistenFn> {
  return events.modsChanged.listen((event) => callback(event.payload))
}

export function onAssetsChanged(callback: (payload: AssetsChanged) => void): Promise<UnlistenFn> {
  return events.assetsChanged.listen((event) => callback(event.payload))
}

export function onGameStatusChanged(
  callback: (payload: GameStatusChanged) => void,
): Promise<UnlistenFn> {
  return events.gameStatusChanged.listen((event) => callback(event.payload))
}

export function onWatchStatusChanged(
  callback: (payload: WatchStatusChanged) => void,
): Promise<UnlistenFn> {
  return events.watchStatusChanged.listen((event) => callback(event.payload))
}

export function onInstallDrop(callback: (payload: InstallDrop) => void): Promise<UnlistenFn> {
  return events.installDrop.listen((event) => callback(event.payload))
}

export function onFolderBusy(callback: (payload: FolderBusy) => void): Promise<UnlistenFn> {
  return events.folderBusy.listen((event) => callback(event.payload))
}
