import { i18n } from '../lib/i18n'
import type { OsKind, TrashKind } from '../lib/ipc/types'
import type { TrashPlace } from './types'

let platformOs: OsKind | null = null

// The OS never changes during a run: startup records it once, so dialogs, toasts and error
// messages outside React all name the same place.
export function rememberPlatformOs(os: OsKind | null): void {
  platformOs = os
}

export function trashPlace(): TrashPlace {
  return platformOs === 'windows' ? 'recycleBin' : 'trash'
}

export function trashPlaceWord(place: TrashPlace = trashPlace()): string {
  return i18n.getFixedT(null, 'mods')(($) => $.trash.place[place])
}

// Done messages name the place the backend reports, not the one the OS suggests.
export function placeOfKind(kind: TrashKind): TrashPlace {
  return kind === 'recycleBin' ? 'recycleBin' : 'trash'
}
