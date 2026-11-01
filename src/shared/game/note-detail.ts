import { game } from '../../locales/en/game'

export type SteamKindKey = keyof typeof game.steamKinds
export type NoteDetailKey = keyof typeof game.notes.details

// `DetectionNote.detail` is a code from the backend (a Steam kind for `steamRoot`, a parse reason
// otherwise), never English. A code the UI does not know is dropped instead of shown raw.
export function isSteamKindKey(value: string): value is SteamKindKey {
  return Object.hasOwn(game.steamKinds, value)
}

export function isNoteDetailKey(value: string): value is NoteDetailKey {
  return Object.hasOwn(game.notes.details, value)
}
