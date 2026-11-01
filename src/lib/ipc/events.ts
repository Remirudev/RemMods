import type { UnlistenFn } from '@tauri-apps/api/event'
import { events } from './bindings'
import type { GameStatusChanged, ModsChanged, WatchStatusChanged } from './types'

// The backend does not buffer events: subscribe first, then fetch what the event would update.
export function onModsChanged(callback: (payload: ModsChanged) => void): Promise<UnlistenFn> {
  return events.modsChanged.listen((event) => callback(event.payload))
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
