import type { ReactNode, RefObject } from 'react'
import type {
  ArchiveEntry,
  AssetEntry,
  ConflictLink,
  ContainerSummary,
  ContentConflictLink,
  ContentSummary,
  DeleteOutcome,
  DeletePrepared,
  DeleteRequest,
  GameStatus,
  InstallInfo,
  LeftoverEntry,
  ModInfo,
  ModIssue,
  ModKind,
  ModRow,
  ModState,
  ModsSnapshot,
  ModTarget,
  RepairItem,
  RepairList,
  RootInfo,
  RunningState,
  TrashOutcome,
  TrashRequest,
  TrashUnavailableReason,
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

export type ModsTab = 'mods' | 'archives' | 'unmanaged' | 'issues' | 'repair'

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

export type NameIssueCode =
  | 'empty'
  | 'dot_name'
  | 'too_long'
  | 'forbidden_char'
  | 'control_char'
  | 'bidi_control'
  | 'trailing_dot_or_space'
  | 'reserved_device_name'
  | 'disabled_prefix'
  | 'temp_name'
  | 'unchanged'
  | 'exists'

export type NavigationKey = 'ArrowUp' | 'ArrowDown' | 'Home' | 'End' | 'PageUp' | 'PageDown'

export type TableKeyIntent =
  | { kind: 'space' }
  | { kind: 'escape' }
  | { kind: 'open' }
  | { kind: 'manage'; action: 'rename' | 'trash' | 'delete' }
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
  onManage: ManageRows
}

export interface ToggleCellProps {
  row: ModRow
  onToggle: ToggleRows
}

export interface RowMenuProps {
  row: ModRow
  onManage: ManageRows
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

export type ManagedKind =
  | 'folder'
  | 'loose'
  | 'linked'
  | 'archiveInRoot'
  | 'archiveInPaks'
  | 'leftover'
  | 'unmanaged'

// What the trash and delete flows need to know about one entry, whatever list it comes from.
export interface ManageSubject {
  target: ModTarget
  name: string
  sizeBytes: number | null
  kind: ManagedKind
}

export type ManageRows = (action: 'rename' | 'trash' | 'delete', row: ModRow) => void

export interface FlowDeps {
  api: {
    trash: (request: TrashRequest) => Promise<TrashOutcome>
    prepare: (target: ModTarget) => Promise<DeletePrepared>
    remove: (request: DeleteRequest) => Promise<DeleteOutcome>
  }
  ask: (dialog: FlowDialog) => Promise<boolean>
  askRunning: () => Promise<boolean>
  store: (snapshot: ModsSnapshot) => void
  notice: (notice: FlowNotice) => void
  fail: (error: unknown) => void
}

export type FlowDialog =
  | { kind: 'trash'; subject: ManageSubject }
  | {
      kind: 'trashUnavailable'
      subject: ManageSubject
      reason: TrashUnavailableReason
      size: number | null
    }
  | { kind: 'deleteFirst'; subject: ManageSubject }
  | { kind: 'deleteFinal'; subject: ManageSubject; name: string; size: number | null }

export type FlowNotice =
  | { kind: 'trashDone'; name: string }
  | { kind: 'deleteDone'; name: string }
  | { kind: 'deleteExpired' }
  | { kind: 'deleteLeftover'; path: string }

export type RepairOperation =
  | { kind: 'rename'; suggestedName: string | null }
  | { kind: 'trash' }
  | { kind: 'delete' }
  | { kind: 'reveal' }
  | { kind: 'toggle'; enable: boolean }
  | { kind: 'logs' }

export interface ConfirmButtonsProps {
  cancelLabel: string
  confirmLabel: string
  danger?: boolean
  // The confirm button stays disabled this long, so a double Enter cannot pass two confirms.
  delayMs?: number
  onCancel: () => void
  onConfirm: () => void
}

export interface DeleteConfirmBodyProps {
  name: string
  size: number | null
  originalApp: boolean
  onCancel: () => void
  onConfirm: () => void
}

export interface TrashUnavailableBodyProps {
  name: string
  size: number | null
  reason: TrashUnavailableReason
  originalApp: boolean
  onCancel: () => void
  onConfirm: () => void
}

export interface RenameDialogProps {
  target: ModTarget
  currentName: string
  initialName: string
  siblings: ReadonlySet<string>
  originalApp: boolean
  onClose: () => void
}

export interface ManageActions {
  rename: (subject: ManageSubject, suggestedName?: string | null) => void
  trash: (subject: ManageSubject) => void
  remove: (subject: ManageSubject) => void
}

export interface MetadataEditorProps {
  row: ModRow
  onDone: () => void
}

export interface MetadataInfoProps {
  row: ModRow
}

export interface DrawerActionsProps {
  row: ModRow
}

export interface RepairTabProps {
  list: RepairList | undefined
}

export interface RepairItemRowProps {
  item: RepairItem
}
