import type { ReactNode, RefObject } from 'react'
import type {
  ArchiveEntry,
  AssetEntry,
  ConflictLink,
  ContainerSummary,
  ContentConflictLink,
  ContentSummary,
  GameStatus,
  InstallInfo,
  LeftoverEntry,
  ModInfo,
  ModIssue,
  ModKind,
  ModRow,
  ModState,
  ModsSnapshot,
  RootInfo,
  RunningState,
  UnmanagedEntry,
  UpdateNotice,
  UpdateTracking,
} from '../../lib/ipc/types'

// PLAN §7: a prefixed LogicMods folder whose paks are still live is "disabled by name only".
export type ModDisplayState = ModState | 'disabledLoaded'

export type ModColumnId =
  | 'name'
  | 'state'
  | 'kind'
  | 'version'
  | 'category'
  | 'size'
  | 'conflicts'
  | 'issues'

export type ModInfoField = keyof ModInfo['defaulted']

export type IssueSeverity = 'error' | 'warning' | 'info'

export type IssueKind = ModIssue['kind']

export type ModsTab = 'mods' | 'archives' | 'unmanaged' | 'issues'

export type StateFilter = 'all' | ModDisplayState

export interface ModFilter {
  text: string
  state: StateFilter
  kinds: readonly ModKind[]
  conflictsOnly: boolean
  contentConflictsOnly: boolean
  issuesOnly: boolean
}

export type KindLabel = (kind: ModKind) => string

export type NavigationKey = 'ArrowUp' | 'ArrowDown' | 'Home' | 'End' | 'PageUp' | 'PageDown'

export type TableKeyIntent =
  | { kind: 'space' }
  | { kind: 'escape' }
  | { kind: 'open' }
  | { kind: 'move'; key: NavigationKey }
  | { kind: 'extend'; key: NavigationKey }

export interface SelectionState {
  selection: string[]
  anchor: string | null
}

export type ModActionId = 'enable' | 'disable'

export interface ModAction {
  id: ModActionId
  count: number
  disabled: boolean
}

// Runs a toggle for these rows; the caller decides nothing else (the backend skips what it must).
export type ToggleRows = (rows: readonly ModRow[], enable: boolean) => void

export interface ModsUiState {
  filter: ModFilter
  tab: ModsTab
  detailsRef: string | null
  activeIndex: number
  selection: string[]
  anchor: string | null
  setText: (text: string) => void
  setStateFilter: (state: StateFilter) => void
  setKinds: (kinds: readonly ModKind[]) => void
  setConflictsOnly: (conflictsOnly: boolean) => void
  setContentConflictsOnly: (contentConflictsOnly: boolean) => void
  setIssuesOnly: (issuesOnly: boolean) => void
  clearFilters: () => void
  toggleConflictsView: () => void
  setTab: (tab: ModsTab) => void
  openDetails: (modRef: string) => void
  closeDetails: () => void
  setActiveIndex: (index: number) => void
  setSelection: (selection: string[], anchor: string | null) => void
  clearSelection: () => void
  reconcileRefs: (previous: readonly ModRow[] | undefined, next: readonly ModRow[]) => void
}

export interface ModTableMeta {
  rowCount: number
}

export interface ModColumnMeta {
  width?: number
  align?: 'left' | 'right'
}

export interface ModsPanelProps {
  status: GameStatus
  install: InstallInfo
}

export interface ModsHeaderProps {
  snapshot: ModsSnapshot | undefined
  running: RunningState
  tracking: UpdateTracking
  refreshing: boolean
  onRefresh: () => void
}

export interface GameControlsProps {
  running: RunningState
  tracking: UpdateTracking
  refreshing: boolean
  onRefresh: () => void
}

export interface LaunchButtonProps {
  running: RunningState
  tracking: UpdateTracking
}

export interface RunningBadgeProps {
  running: RunningState
}

export interface UpdateNoticeProps {
  update: UpdateNotice
}

export interface RootNoticesProps {
  roots: readonly RootInfo[]
}

export interface ConflictsChipProps {
  rows: readonly ModRow[]
}

export interface ModsTabsProps {
  snapshot: ModsSnapshot
  paks: string
  searchRef: RefObject<HTMLInputElement | null>
}

export interface ModFiltersProps {
  shown: number
  total: number
  searchRef: RefObject<HTMLInputElement | null>
}

export interface ModTableProps {
  rows: readonly ModRow[]
  onOpen: (modRef: string) => void
  onToggle: ToggleRows
}

export interface ToggleCellProps {
  row: ModRow
  onToggle: ToggleRows
}

export interface ActionBarProps {
  rows: readonly ModRow[]
  onToggle: ToggleRows
}

export interface ModRowCellProps {
  row: ModRow
}

export interface ModDetailsDrawerProps {
  rows: readonly ModRow[]
}

export interface DetailsFilesProps {
  modRef: string
}

export interface ConflictListProps {
  conflicts: readonly ConflictLink[]
}

export interface ArchivesListProps {
  archives: readonly ArchiveEntry[]
}

export interface UnmanagedListProps {
  entries: readonly UnmanagedEntry[]
}

export interface IssuesListProps {
  rows: readonly ModRow[]
  roots: readonly RootInfo[]
  leftovers: readonly LeftoverEntry[]
}

export interface IssueTextProps {
  issue: ModIssue
  root: string
}

export interface RevealButtonProps {
  modRef: string
  label: string
}

export interface ListSectionProps {
  description: string
  empty: string
  count: number
  children: ReactNode
}

export interface ModsListTabProps {
  rows: readonly ModRow[]
  paks: string
  searchRef: RefObject<HTMLInputElement | null>
}

export interface DrawerSectionProps {
  title: string
  children: ReactNode
}

export interface ModsErrorAlertProps {
  error: unknown
  onRetry: () => void
}

export interface ContentSectionProps {
  row: ModRow
}

export interface ContentListProps {
  entries: readonly AssetEntry[]
}

export interface ContentHintsProps {
  content: ContentSummary
}

export interface UnsupportedContainersProps {
  containers: readonly ContainerSummary[]
}

export interface ContentConflictListProps {
  conflicts: readonly ContentConflictLink[]
}
