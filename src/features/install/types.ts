import type {
  Inspection,
  InstallCandidate,
  InstallIssue,
  InstallIssueCode,
  InstallOutcome,
  InstallProgress,
  IssueSeverity,
  NameEncoding,
  NameOverride,
  RootKind,
} from '../../lib/ipc/types'
import type { QueuedSource } from '../../shared/types'

export interface InstallChoices {
  selected: number[]
  // Candidates whose "replace the installed mod" switch is on.
  replace: number[]
  // Case clash id -> index of the path to keep; absent while unresolved.
  clashKeeps: Record<number, number>
  autoFix: boolean
}

export type StartBlocker =
  | 'blocked'
  | 'noSelection'
  | 'ambiguous'
  | 'replaceNotConfirmed'
  | 'unresolvedClash'
  | 'invalidNames'
  | 'noSpace'

export interface InspectOptions {
  encoding: NameEncoding
  root: RootKind | null
  names: NameOverride[]
}

export type FlowStage =
  | 'inspecting'
  | 'preview'
  | 'confirmRunning'
  | 'running'
  | 'done'
  | 'cancelled'
  | 'failed'

export type PreviewNotice = 'stale' | 'busy'

export interface InstallFlow {
  stage: FlowStage
  options: InspectOptions
  inspection: Inspection | null
  choices: InstallChoices | null
  notice: PreviewNotice | null
  outcome: InstallOutcome | null
  error: unknown
  reinspecting: boolean
  progress: InstallProgress | null
  setRoot: (root: RootKind) => void
  setEncoding: (encoding: NameEncoding) => void
  setName: (id: number, name: string) => void
  setChoices: (choices: InstallChoices) => void
  start: (applyWhileRunning?: boolean) => void
  cancel: () => void
  backToPreview: () => void
}

export interface IssueGroup {
  severity: IssueSeverity
  issues: InstallIssue[]
}

export type IssueTextKey = InstallIssueCode

export interface InstallModalProps {
  source: QueuedSource
  // Sources waiting behind this one.
  remaining: number
  // Closes this modal and moves on to the next source.
  onClose: () => void
  // Drops this source and everything waiting behind it.
  onStop: () => void
}

export interface InstallPreviewProps {
  flow: InstallFlow
  inspection: Inspection
  choices: InstallChoices
  onClose: () => void
}

export interface CandidateListProps {
  inspection: Inspection
  choices: InstallChoices
  onChoices: (choices: InstallChoices) => void
  onName: (id: number, name: string) => void
}

export interface CandidateRowProps {
  candidate: InstallCandidate
  inVariantGroup: boolean
  chosen: boolean
  replace: boolean
  autoFix: boolean
  onToggle: () => void
  onReplace: (replace: boolean) => void
  onName: (name: string) => void
}

export interface RootSelectProps {
  inspection: Inspection
  value: RootKind
  disabled: boolean
  onChange: (root: RootKind) => void
}

export interface EncodingSelectProps {
  value: NameEncoding
  disabled: boolean
  onChange: (encoding: NameEncoding) => void
}

export interface IssueListProps {
  issues: readonly InstallIssue[]
}

export interface CaseClashPickerProps {
  inspection: Inspection
  choices: InstallChoices
  onChoices: (choices: InstallChoices) => void
}

export interface InstallProgressViewProps {
  progress: InstallProgress | null
  onCancel: () => void
}

export interface InstallResultProps {
  // Null after a cancel: nothing was installed.
  outcome: InstallOutcome | null
  remaining: number
  onClose: () => void
  onStop: () => void
}

export interface RunningConfirmProps {
  onConfirm: () => void
  onBack: () => void
}

export interface InstallFailureProps {
  error: unknown
  canGoBack: boolean
  remaining: number
  onBack: () => void
  onClose: () => void
  onStop: () => void
}
