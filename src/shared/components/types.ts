import type { ReactNode } from 'react'
import type { DetectionNote, FolderProblem, GameCandidate, GameStatus } from '../../lib/ipc/types'
import type { FolderChooser } from '../game/types'

export interface PanelHeaderProps {
  title: string
  description?: string
  actions?: ReactNode
}

export interface EmptyStateProps {
  icon: ReactNode
  title: string
  description: string
}

export interface GameLocationChooserProps {
  status: GameStatus
  chooser: FolderChooser
  withActions: boolean
}

export interface GameCandidateListProps {
  candidates: readonly GameCandidate[]
  onUse: (id: string) => void
  busyId: string | null
}

export interface ManualPathFormProps {
  onSubmitted?: () => void
}

export interface DetectionNotesProps {
  notes: readonly DetectionNote[]
}

export interface InvalidFolderAlertProps {
  reason: FolderProblem
  path: string
}

export interface GameCandidateItemProps {
  candidate: GameCandidate
  onUse: () => void
  busy: boolean
}
