import type { SourceKind } from '../lib/ipc/types'

export type UiScaleAction = 'increase' | 'decrease' | 'reset'

export type UiScaleKeyEvent = Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'altKey' | 'metaKey'>

export type QueuedSource =
  | { kind: 'token'; id: string; name: string; sourceKind: SourceKind }
  | { kind: 'archive'; row: string; name: string }

export interface PictureDropTarget {
  // What the overlay says while pictures are dragged over the window.
  label: string
  // A set takes every picture of a drop, a mod only the first.
  many: boolean
  apply: (sourceId: string) => Promise<void>
}

export interface PictureDropState {
  target: PictureDropTarget | null
  setTarget: (target: PictureDropTarget) => void
  clearTarget: () => void
}

export interface InstallQueueState {
  queue: readonly QueuedSource[]
  // The head of the queue, the one being installed.
  current: QueuedSource | null
  // Counts head changes, so a modal for the same source twice in a row is still a new modal.
  turn: number
  push: (sources: readonly QueuedSource[]) => void
  shift: () => void
  clear: () => void
}

export interface UpdateRunState {
  updating: boolean
  setUpdating: (updating: boolean) => void
}

export type TrashPlace = 'trash' | 'recycleBin'

// A group whose mods the Mods list shows on their own: the keys the group switches on.
export interface GroupFilter {
  id: string
  name: string
  keys: readonly string[]
}

export interface GroupFilterState {
  group: GroupFilter | null
  showGroup: (group: GroupFilter) => void
  clearGroup: () => void
}
