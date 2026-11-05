import type { ReactNode } from 'react'
import type {
  ApplyDirection,
  ApplyOutcome,
  ApplyRequest,
  DiffChange,
  DiffSkip,
  ModRow,
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

export interface SnapshotListProps {
  view: ProfilesView
  snapshots: ProfileSummary[]
}

export interface ProfileDiffModalProps {
  profile: Profile
  direction: ApplyDirection
  onClose: () => void
  // The modal that hosts the diff, locked while the apply runs.
  modalId?: string
}

export interface DiffSectionProps {
  title: string
  count: number
  children: ReactNode
}

export interface TransferItemsProps {
  readOnly: boolean
}

// What a group does with one mod: switch it on, switch it off, or leave it alone.
export type ModPick = 'on' | 'off' | 'skip'

export type Picks = ReadonlyMap<string, ModPick>

// One line of the group dialog's mod list; `on` is null for a listed mod that is not installed.
export interface PickItem {
  key: string
  name: string
  on: boolean | null
}

export interface PickCounts {
  on: number
  off: number
}

// Two mods of one group that replace the same files: only one of them shows in game.
export interface Clash {
  first: string
  second: string
}

// A group as the sidebar shows it.
export interface GroupItem {
  summary: ProfileSummary
  title: string
  // The group's switch: on while the mods match what the group switches on.
  lit: boolean
  onKeys: readonly string[]
  clashes: readonly Clash[]
}

export interface GroupPrefsState {
  askBeforeSwitch: boolean
  collapsed: boolean
  setAskBeforeSwitch: (ask: boolean) => void
  setCollapsed: (collapsed: boolean) => void
}

export type ApplyMutate = (request: ApplyRequest) => Promise<ApplyOutcome>

export interface ApplyHandlers {
  onDone: (outcome: ApplyOutcome) => void
  // The disk changed since the diff was fetched; nothing was applied.
  onStale: () => void
}

export interface GroupsSidebarProps {
  rows: readonly ModRow[]
}

export interface GroupRowProps {
  item: GroupItem
  rows: readonly ModRow[]
  active: boolean
  busy: boolean
  readOnly: boolean
  onSwitch: (item: GroupItem) => void
  onShow: (item: GroupItem) => void
}

export interface GroupRowMenuProps {
  item: GroupItem
  rows: readonly ModRow[]
  readOnly: boolean
}

export interface CollapsedGroupsProps {
  items: readonly GroupItem[]
  busyId: string | null
  readOnly: boolean
  onSwitch: (item: GroupItem) => void
}

export interface GroupsMenuProps {
  readOnly: boolean
}

export interface GroupDialogProps {
  // null creates a new group.
  profile: Profile | null
  rows: readonly ModRow[]
  onClose: () => void
  modalId?: string
}

export interface ModPickerProps {
  items: readonly PickItem[]
  picks: Picks
  onPick: (keys: readonly string[], pick: ModPick) => void
  onFromCurrent: () => void
}

export interface ModPickRowProps {
  item: PickItem
  pick: ModPick
  onPick: (keys: readonly string[], pick: ModPick) => void
}

export interface ClashAlertProps {
  clashes: readonly Clash[]
}

export interface ApplyToastProps {
  notificationId: string
  outcome: ApplyOutcome
}
