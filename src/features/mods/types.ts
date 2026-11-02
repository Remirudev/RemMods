import type { Header } from '@tanstack/react-table'
import type { ReactNode, RefObject } from 'react'
import type {
  ArchiveEntry,
  AssetEntry,
  BulkDeletePrepared,
  BulkDeleteRequest,
  BulkOutcome,
  BulkRow,
  BulkTrashRequest,
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
  PreparedRow,
  RepairItem,
  RepairList,
  RootInfo,
  RunningState,
  TrashKind,
  TrashOutcome,
  TrashRequest,
  TrashUnavailableReason,
  UnmanagedEntry,
  UpdateNotice,
  UpdateTracking,
} from '../../lib/ipc/types'
import type { modTableFeatures } from './mod-table-features'

export type ModTableHeader = Header<typeof modTableFeatures, ModRow, unknown>

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
  | 'contentConflicts'
  | 'issues'

// Name always stays visible, like the switch and the row menu, which are not data columns.
export type HideableColumnId = Exclude<ModColumnId, 'name'>

export interface ColumnSizeLimits {
  size: number
  minSize: number
  maxSize: number
}

// The shapes TanStack Table takes as `columnSizing` and `columnVisibility`. A column missing from
// `sizing` has its default width; for Name that means it fills the free space.
export interface ColumnLayout {
  sizing: Record<string, number>
  visibility: Record<string, boolean>
}

export interface ColumnLayoutState extends ColumnLayout {
  // Not saved: a drag calls it on every move and `saveLayout` once at the end.
  setWidth: (id: ModColumnId, width: number) => void
  saveLayout: () => void
  resetWidth: (id: ModColumnId) => void
  setColumnVisible: (id: HideableColumnId, visible: boolean) => void
  resetLayout: () => void
}

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

export type ModActionId = 'enable' | 'disable' | 'trash' | 'delete'

export type BulkAction = 'trash' | 'delete'

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
  align?: 'left' | 'right'
  // Data columns only; the switch and the row menu keep their width.
  resizable?: boolean
}

export interface ModHeaderCellProps {
  header: ModTableHeader
  // Name without a stored width fills the free space, so its drag starts from what is drawn.
  fills: boolean
}

export interface ColumnResizeHandleProps {
  label: string
  width: number
  minWidth: number
  maxWidth: number
  // The width a drag or key press starts from.
  measure: () => number
  onResize: (width: number) => void
  onCommit: () => void
  onReset: () => void
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
  onBulk: (action: BulkAction) => void
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
  onTrash: (rows: readonly ModRow[]) => void
  onDelete: (rows: readonly ModRow[]) => void
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
  | { kind: 'trashDone'; name: string; trash: TrashKind }
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
  otherManager: boolean
  onCancel: () => void
  onConfirm: () => void
}

export interface BulkSelection {
  subjects: ManageSubject[]
  skipped: number
}

export interface BulkDeps {
  api: {
    trashMany: (request: BulkTrashRequest) => Promise<BulkOutcome>
    prepareMany: (targets: ModTarget[]) => Promise<BulkDeletePrepared>
    deleteMany: (request: BulkDeleteRequest) => Promise<BulkOutcome>
  }
  ask: (dialog: BulkDialog) => Promise<boolean>
  askRunning: () => Promise<boolean>
  store: (snapshot: ModsSnapshot | null) => void
  notice: (notice: BulkNotice) => void
  showResults: (results: BulkResults) => Promise<'deleteUnavailable' | 'close'>
  fail: (error: unknown) => void
}

export type BulkDialog =
  | { kind: 'bulkTrash'; subjects: ManageSubject[]; skipped: number }
  | { kind: 'bulkDeleteFirst'; subjects: ManageSubject[]; skipped: number }
  | { kind: 'bulkDeleteFinal'; rows: PreparedRow[] }

export type BulkNotice =
  | { kind: 'bulkTrashDone'; count: number; trash: TrashKind }
  | { kind: 'bulkDeleteDone'; count: number }
  | { kind: 'bulkNothingEligible' }
  | { kind: 'deleteExpired' }

/** `subjects` (trash only): the subjects sent, so the dialog can apply `canManage(…, 'delete')`. */
export interface BulkResults {
  action: 'trash' | 'delete'
  rows: BulkRow[]
  subjects?: ManageSubject[]
}

export interface BulkActions {
  trash: (rows: readonly ModRow[]) => void
  remove: (rows: readonly ModRow[]) => void
}

// One row of a size list: what a dialog shows before a permanent delete.
export interface SizedName {
  key: string
  name: string
  size: number | null
  note?: string
}

export interface BulkConfirmBodyProps {
  lines: string[]
  names: readonly { key: string; name: string }[]
  more: number
  confirmLabel: string
  danger: boolean
  onCancel: () => void
  onConfirm: () => void
}

export interface BulkFinalBodyProps {
  rows: readonly PreparedRow[]
  onCancel: () => void
  onConfirm: () => void
}

export interface BulkDeleteWarningProps {
  heading?: string
  rows: readonly SizedName[]
  footer: string
}

export interface BulkRowStatus {
  icon: ReactNode
  color: string
  text: string
}

export interface BulkResultListProps {
  rows: readonly BulkRow[]
  subjects?: readonly ManageSubject[]
}

export interface BulkResultsBodyProps {
  results: BulkResults
  onClose: () => void
  onDelete: () => void
}

export interface TrashUnavailableBodyProps {
  name: string
  size: number | null
  reason: TrashUnavailableReason
  otherManager: boolean
  onCancel: () => void
  onConfirm: () => void
}

export interface RenameDialogProps {
  target: ModTarget
  currentName: string
  initialName: string
  siblings: ReadonlySet<string>
  otherManager?: boolean
  onClose: () => void
  // The modal that hosts the dialog, locked while the rename runs.
  modalId?: string
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
