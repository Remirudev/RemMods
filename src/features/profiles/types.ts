import type { ReactNode } from 'react'
import type {
  DiffChange,
  DiffSkip,
  Profile,
  ProfileMode,
  ProfileSave,
  ProfileSummary,
  ProfilesView,
} from '../../lib/ipc/types'

export interface DiffGroups {
  enable: DiffChange[]
  disable: DiffChange[]
  skipped: DiffSkip[]
  missing: string[]
  hasChanges: boolean
}

export interface SplitProfiles {
  user: ProfileSummary[]
  snapshots: ProfileSummary[]
}

export interface ProfileFormValues {
  name: string
  description: string
  mode: ProfileMode
}

export interface ProfileFormProps {
  initial: ProfileFormValues
  submitLabel: string
  buildSave: (values: ProfileFormValues) => ProfileSave
  onClose: () => void
}

export interface SaveProfileDialogProps {
  onClose: () => void
}

export interface EditProfileDialogProps {
  profile: Profile
  onClose: () => void
}

export interface ProfileCardProps {
  summary: ProfileSummary
  view: ProfilesView
}

export interface ProfileListProps {
  view: ProfilesView
  profiles: ProfileSummary[]
}

export interface SnapshotListProps {
  view: ProfilesView
  snapshots: ProfileSummary[]
}

export interface ProfileDiffModalProps {
  profile: Profile
  onClose: () => void
}

export interface DiffSectionProps {
  title: string
  count: number
  children: ReactNode
}

export interface TransferMenuProps {
  readOnly: boolean
}
