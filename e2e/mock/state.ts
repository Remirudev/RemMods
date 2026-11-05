import type {
  AppError,
  ArchiveEntry,
  ModRow,
  ModsSnapshot,
  ModTarget,
} from '../../src/lib/ipc/types'
import {
  makeCandidate,
  makeGameStatus,
  makeModRow,
  makeSnapshot,
  TEST_INSTALL,
  TEST_PLATFORM_INFO,
  TEST_SETTINGS,
  WINDOWS_PLATFORM_INFO,
} from '../../src/test/fixtures'
import type { MockState, Scenario } from './types'

const MIB = 1024 * 1024

export const ARCHIVE: ArchiveEntry = {
  modRef: 'a:mods/echo.zip',
  relPath: 'Mods/Echo.zip',
  sizeBytes: 4096,
}

export const DETECTED = makeCandidate({ id: 'candidate-1' })

// Handlers throw the AppError object itself: the mocked invoke rejects with it, and the typed
// bindings turn that into their `{ status: "error" }` result.
export function fail(error: AppError): never {
  throw error
}

export function parseScenario(value: string | null): Scenario {
  return value === 'onboarding' || value === 'windows' ? value : 'ready'
}

function initialRows(): ModRow[] {
  return [
    makeModRow('Alpha', { sizeBytes: 3 * MIB }),
    makeModRow('Bravo', { sizeBytes: 5 * MIB }),
    makeModRow('Charlie', {
      state: 'disabled',
      payload: { livePak: 0, offPak: 1, liveCompanion: 0, offCompanion: 0 },
    }),
    makeModRow('Delta'),
    makeModRow('Foxtrot', {
      kind: 'logic',
      root: 'LogicMods',
      location: `${TEST_INSTALL.paks}/LogicMods/Foxtrot`,
    }),
  ]
}

export function createState(scenario: Scenario): MockState {
  const windows = scenario === 'windows'
  return {
    scenario,
    settings: structuredClone(TEST_SETTINGS),
    platform: structuredClone(windows ? WINDOWS_PLATFORM_INFO : TEST_PLATFORM_INFO),
    game:
      scenario === 'onboarding'
        ? makeGameStatus({ location: { kind: 'notSet' } })
        : makeGameStatus(),
    rows: initialRows(),
    archives: [{ ...ARCHIVE }],
    revision: 1,
    profiles: [],
    lastApplied: null,
    nextId: 1,
    tokens: [],
    issuedTokens: [],
    trashKind: windows ? 'recycleBin' : 'home',
    trashRefusals: { bravo: { reason: windows ? 'recycleRefused' : 'otherDevice' } },
    busyKeys: ['delta'],
    waits: [],
    inspections: [],
    trashed: [],
    deleted: [],
    ownPictures: {},
  }
}

export function nextId(state: MockState, prefix: string): string {
  const id = `${prefix}-${state.nextId}`
  state.nextId += 1
  return id
}

export function snapshotOf(state: MockState): ModsSnapshot {
  return structuredClone(
    makeSnapshot(state.rows, {
      revision: state.revision,
      scannedAt: new Date().toISOString(),
      archives: state.archives,
    }),
  )
}

// Every change on disk gets a new revision, like a real rescan.
export function changed(state: MockState): ModsSnapshot {
  state.revision += 1
  return snapshotOf(state)
}

export function findRow(state: MockState, modRef: string): ModRow | undefined {
  return state.rows.find((row) => row.modRef === modRef)
}

// Core refuses a target as stale when its ref and key no longer name the same row.
export function rowFor(state: MockState, target: ModTarget): ModRow {
  const row = findRow(state, target.modRef)
  if (!row || row.key !== target.key) fail({ kind: 'stale' })
  return row
}

export function removeRow(state: MockState, row: ModRow): void {
  state.rows = state.rows.filter((candidate) => candidate !== row)
}

export function targetOf(row: ModRow): ModTarget {
  return { modRef: row.modRef, key: row.key }
}

export function isEnabled(row: ModRow): boolean {
  return row.state === 'enabled'
}

export function setEnabled(row: ModRow, enable: boolean): void {
  const pakFiles = row.payload.livePak + row.payload.offPak
  row.state = enable ? 'enabled' : 'disabled'
  row.payload = {
    ...row.payload,
    livePak: enable ? pakFiles : 0,
    offPak: enable ? 0 : pakFiles,
  }
}
