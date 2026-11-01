import type {
  ContentSummary,
  GameCandidate,
  GameStatus,
  InstallInfo,
  ModContents,
  ModDetails,
  ModInfo,
  ModRow,
  ModsSnapshot,
  PlatformInfo,
  Settings,
} from '../lib/ipc/types'

export const TEST_SETTINGS: Settings = {
  version: 1,
  appearance: {
    scheme: 'dark',
    uiScale: 1,
    titleBar: 'custom',
  },
  readOnly: false,
  recovered: false,
}

export const TEST_PLATFORM_INFO: PlatformInfo = {
  os: 'linux',
  arch: 'x86_64',
  appVersion: '0.1.0',
  webviewVersion: '2.52.0',
  smoke: false,
  paths: {
    config: '/home/test/.config/com.remirudev.remmods',
    data: '/home/test/.local/share/com.remirudev.remmods',
    logs: '/home/test/.local/share/com.remirudev.remmods/logs',
  },
}

const PAKS = '/games/steamapps/common/TEKKEN 8/Polaris/Content/Paks'

export const TEST_INSTALL: InstallInfo = {
  root: '/games/steamapps/common/TEKKEN 8',
  paks: PAKS,
  source: 'steam',
  steamKind: 'native',
  buildId: '25221600',
  updateTracking: 'available',
}

export function makeGameStatus(overrides: Partial<GameStatus> = {}): GameStatus {
  return {
    location: { kind: 'ready', install: TEST_INSTALL },
    running: 'notRunning',
    session: 0,
    originalApp: false,
    update: null,
    candidates: [],
    notes: [],
    ...overrides,
  }
}

export function makeCandidate(overrides: Partial<GameCandidate> = {}): GameCandidate {
  return {
    id: 'candidate-1',
    root: '/games/steamapps/common/TEKKEN 8',
    library: '/games',
    steamKind: 'native',
    buildId: '25221600',
    fullyInstalled: true,
    problem: null,
    inUse: false,
    ...overrides,
  }
}

export const TEST_INFO: ModInfo = {
  version: '1.0',
  category: 'Misc',
  description: '',
  author: 'Unknown',
  origin: 'missing',
  defaulted: { version: true, category: true, description: true, author: true },
}

export const EMPTY_CONTENT: ContentSummary = {
  containers: [],
  assets: 0,
  characters: [],
  slots: [],
  internalOverlaps: 0,
}

export function makeModRow(name: string, overrides: Partial<ModRow> = {}): ModRow {
  const key = name.toLowerCase()
  return {
    modRef: `m:${key}`,
    key,
    kind: 'standard',
    name,
    folderName: name,
    root: 'Mods',
    location: `${PAKS}/Mods/${name}`,
    state: 'enabled',
    logicPrefixed: false,
    payload: { livePak: 1, offPak: 0, liveCompanion: 0, offCompanion: 0 },
    sizeBytes: 2048,
    info: TEST_INFO,
    conflictsWith: [],
    content: EMPTY_CONTENT,
    contentConflicts: [],
    ambiguous: false,
    toggleBlock: null,
    issues: [],
    ...overrides,
  }
}

export function makeSnapshot(rows: ModRow[], overrides: Partial<ModsSnapshot> = {}): ModsSnapshot {
  return {
    revision: 1,
    scannedAt: '2026-11-01T10:00:00Z',
    paks: PAKS,
    roots: [],
    rows,
    archives: [],
    unmanaged: [],
    leftovers: [],
    stats: {
      dirsRead: 3,
      entriesSeen: rows.length,
      durationMs: 2,
      containersRead: 0,
      containersCached: 0,
    },
    ...overrides,
  }
}

export const TEST_ROWS: ModRow[] = [
  makeModRow('Alpha Skin', { sizeBytes: 3 * 1024 * 1024 }),
  makeModRow('Beta Stage', {
    state: 'disabled',
    payload: { livePak: 0, offPak: 1, liveCompanion: 0, offCompanion: 0 },
    conflictsWith: [
      { modRef: 'm:alpha skin', key: 'alpha skin', name: 'Alpha Skin', paks: ['a.pak'] },
    ],
    issues: [{ kind: 'linkedMod' }],
  }),
  makeModRow('Gamma Logic', { kind: 'logic', root: 'LogicMods' }),
]

export const TEST_SNAPSHOT: ModsSnapshot = makeSnapshot(TEST_ROWS)

export const TEST_DETAILS: ModDetails = {
  modRef: 'm:alpha skin',
  location: `${PAKS}/Mods/Alpha Skin`,
  files: [{ relPath: 'alpha.pak', kind: 'pak', live: true, sizeBytes: 2048 }],
  truncated: false,
  otherFiles: 0,
}

export const TEST_CONTENTS: ModContents = {
  modRef: 'm:alpha skin',
  base: 'complete',
  entries: [],
  total: 0,
  truncated: false,
}

export const INSTALL_WITHOUT_MANIFEST: Pick<InstallInfo, 'buildId' | 'updateTracking'> = {
  buildId: null,
  updateTracking: 'noManifest',
}
