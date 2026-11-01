import type { FolderProblem, GameStatusChanged } from '../../lib/ipc/types'

// What `game-status-changed` carries: the monitor's view, never the location.
export type GameStatusEvent = GameStatusChanged

export interface FolderChooser {
  pick: () => void
  picking: boolean
  cancelled: boolean
  invalid: { reason: FolderProblem; path: string } | null
}
