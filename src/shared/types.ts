import type { PointerEvent } from 'react'
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

export interface ClickModifiers {
  shift: boolean
  ctrl: boolean
}

export interface SelectionState {
  selection: string[]
  anchor: string | null
}

// A group the Mods list can narrow to: the keys the group switches on.
export interface GroupChoice {
  id: string
  name: string
  keys: readonly string[]
}

export interface GroupFilterState {
  // Every group, in the sidebar's order.
  groups: readonly GroupChoice[]
  // The groups whose mods the list shows together; none shows every mod.
  picked: readonly string[]
  // Where a Shift+click range starts.
  anchor: string | null
  setGroups: (groups: readonly GroupChoice[]) => void
  pickGroups: (picked: readonly string[], anchor: string | null) => void
  clearPicks: () => void
}

export interface PictureSize {
  width: number
  height: number
}

// The on-screen picture a big one opens from.
export interface PreviewAnchor {
  left: number
  right: number
  top: number
  bottom: number
}

export interface PicturePreview {
  // The useId of the picture that asked, so it only ever closes its own preview.
  owner: string
  src: string
  natural: PictureSize
  anchor: PreviewAnchor
}

export interface PicturePreviewState {
  preview: PicturePreview | null
  showPreview: (preview: PicturePreview) => void
  hidePreview: () => void
}

export interface PreviewBox extends PictureSize {
  left: number
  top: number
}

export interface PicturePreviewHandlers {
  onPointerEnter: (event: PointerEvent<HTMLElement>) => void
  onPointerLeave: () => void
  onPointerDown: () => void
  onWheel: () => void
}
