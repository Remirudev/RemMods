import type {
  Inspection,
  InstallCandidate,
  InstallIssue,
  InstallOutcome,
  InstallProgress,
} from '../lib/ipc/types'
import { makeModRow, makeSnapshot } from './fixtures'

export function makeCandidate(
  id: number,
  name: string,
  overrides: Partial<InstallCandidate> = {},
): InstallCandidate {
  return {
    id,
    name,
    inner: '',
    files: 2,
    payload: 1,
    shippedOff: 0,
    bytes: 4096,
    group: null,
    selected: true,
    nameIssue: null,
    fixedName: null,
    existing: null,
    alsoIn: [],
    pathTooLong: false,
    ...overrides,
  }
}

export function makeIssue(
  code: InstallIssue['code'],
  severity: InstallIssue['severity'],
  overrides: Partial<InstallIssue> = {},
): InstallIssue {
  return { severity, code, path: '', count: 1, ...overrides }
}

export function makeInspection(
  candidates: InstallCandidate[],
  overrides: Partial<Inspection> = {},
): Inspection {
  return {
    id: `insp-${'a'.repeat(32)}`,
    sourceName: 'cool-mod.zip',
    format: 'zip',
    encoding: 'auto',
    legacyNames: false,
    entries: 3,
    files: candidates.reduce((sum, candidate) => sum + candidate.files, 0),
    totalBytes: 8192,
    detectedRoot: 'standard',
    rootReason: 'default',
    root: 'standard',
    candidates,
    groups: [],
    clashes: [],
    issues: [],
    blocked: false,
    availableBytes: 10 * 1024 * 1024 * 1024,
    ...overrides,
  }
}

export function makeOutcome(
  names: string[],
  overrides: Partial<InstallOutcome> = {},
): InstallOutcome {
  return {
    snapshot: makeSnapshot(
      names.map((name) => makeModRow(name)),
      { revision: 5 },
    ),
    installed: names.map((name) => ({
      key: `Standard:${name}`,
      modRef: `m:${name.toLowerCase()}`,
      rel: `Mods/${name}`,
      files: 2,
      replaced: null,
    })),
    ...overrides,
  }
}

export const PROGRESS_EXTRACTING: InstallProgress = {
  phase: 'extracting',
  doneBytes: 2048,
  totalBytes: 4096,
  doneFiles: 1,
  totalFiles: 2,
}

export const PROGRESS_COMMITTING: InstallProgress = {
  phase: 'committing',
  doneBytes: 4096,
  totalBytes: 4096,
  doneFiles: 2,
  totalFiles: 2,
}
