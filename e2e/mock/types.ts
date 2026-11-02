import type {
  ArchiveEntry,
  GameStatus,
  Inspection,
  LastApplied,
  ModRow,
  ModTarget,
  PlatformInfo,
  Profile,
  Settings,
  TrashKind,
  TrashUnavailableReason,
} from '../../src/lib/ipc/types'

export type Scenario = 'ready' | 'onboarding' | 'windows'

export interface RecordedCall {
  cmd: string
  args: unknown
}

export interface IssuedToken {
  token: string
  targets: ModTarget[]
}

export interface BusyWait {
  waitId: number
  key: string
  folder: string
}

export interface TrashRefusal {
  reason: TrashUnavailableReason
}

// Plain data only: specs read it through `page.evaluate`, which copies JSON-like values.
export interface MockState {
  scenario: Scenario
  settings: Settings
  platform: PlatformInfo
  game: GameStatus
  rows: ModRow[]
  archives: ArchiveEntry[]
  revision: number
  profiles: Profile[]
  lastApplied: LastApplied | null
  nextId: number
  tokens: IssuedToken[]
  issuedTokens: string[]
  trashKind: TrashKind
  trashRefusals: Record<string, TrashRefusal>
  busyKeys: string[]
  waits: BusyWait[]
  inspections: Inspection[]
  trashed: string[]
  deleted: string[]
}

export type EventEmitter = (event: string, payload: unknown) => Promise<void>

export type CommandArgs = Record<string, unknown>

export interface E2eHandle {
  calls: RecordedCall[]
  state: MockState
  unknown: string[]
  emit: EventEmitter
  // Ends the install that `install_start` holds open (see install-flow.ts).
  releaseInstall: () => void
}

export interface Backend {
  handle: E2eHandle
  route: (cmd: string, args: CommandArgs) => unknown
}

declare global {
  interface Window {
    __REMMODS_E2E__: E2eHandle
  }
}
