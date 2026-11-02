import type {
  ApplyOutcome,
  ApplyRequest,
  DiffChange,
  EntryState,
  Profile,
  ProfileDiff,
  ProfileEntryDto,
  ProfileSave,
  ProfileSummary,
  ProfilesView,
} from '../../src/lib/ipc/types'
import { makeProfile } from '../../src/test/profile-fixtures'
import { changed, fail, isEnabled, nextId, setEnabled } from './state'
import type { MockState } from './types'

function entryState(enabled: boolean): EntryState {
  return enabled ? 'enabled' : 'disabled'
}

// What a save records: every row that can be toggled, sorted by key.
function captureCurrent(state: MockState): ProfileEntryDto[] {
  return state.rows
    .filter((row) => row.toggleBlock === null)
    .map((row) => ({ key: row.key, state: entryState(isEnabled(row)) }))
    .sort((a, b) => a.key.localeCompare(b.key))
}

function summaryOf(state: MockState, profile: Profile): ProfileSummary {
  const present = profile.mods.filter((entry) => state.rows.some((row) => row.key === entry.key))
  const matchesCurrent = present.every((entry) => {
    const row = state.rows.find((candidate) => candidate.key === entry.key)
    return row !== undefined && entryState(isEnabled(row)) === entry.state
  })
  return {
    profile: structuredClone(profile),
    enabled: profile.mods.filter((entry) => entry.state === 'enabled').length,
    disabled: profile.mods.filter((entry) => entry.state === 'disabled').length,
    missing: profile.mods.length - present.length,
    matchesCurrent,
  }
}

export function profilesView(state: MockState): ProfilesView {
  return {
    profiles: state.profiles.map((profile) => summaryOf(state, profile)),
    lastApplied: state.lastApplied ? { ...state.lastApplied } : null,
    readOnly: false,
    problem: null,
  }
}

function profileById(state: MockState, id: string): Profile {
  return state.profiles.find((profile) => profile.id === id) ?? fail({ kind: 'profileNotFound' })
}

function checkName(state: MockState, name: string, except: string | null): string {
  const trimmed = name.trim()
  if (trimmed === '' || trimmed.length > 100) fail({ kind: 'invalidInput', field: 'name' })
  const taken = state.profiles.some(
    (profile) => profile.id !== except && profile.name.toLowerCase() === trimmed.toLowerCase(),
  )
  if (taken) fail({ kind: 'profileNameTaken' })
  return trimmed
}

export function saveProfile(state: MockState, save: ProfileSave): ProfilesView {
  const now = new Date().toISOString()
  if (save.kind === 'new') {
    const name = checkName(state, save.name, null)
    state.profiles.push(
      makeProfile(nextId(state, 'profile'), {
        name,
        description: save.description,
        mode: save.mode,
        mods: captureCurrent(state),
        created: now,
        updated: now,
      }),
    )
  } else if (save.kind === 'overwrite') {
    const profile = profileById(state, save.id)
    profile.mods = captureCurrent(state)
    profile.updated = now
  } else {
    const profile = profileById(state, save.id)
    profile.name = checkName(state, save.name, profile.id)
    profile.description = save.description
    profile.mode = save.mode
    profile.updated = now
  }
  return profilesView(state)
}

export function deleteProfile(state: MockState, id: string): ProfilesView {
  profileById(state, id)
  state.profiles = state.profiles.filter((profile) => profile.id !== id)
  return profilesView(state)
}

function changesFor(state: MockState, profile: Profile): DiffChange[] {
  const changes: DiffChange[] = []
  for (const entry of profile.mods) {
    const row = state.rows.find((candidate) => candidate.key === entry.key)
    const enable = entry.state === 'enabled'
    if (row && row.toggleBlock === null && isEnabled(row) !== enable) {
      changes.push({ modRef: row.modRef, key: row.key, name: row.name, enable, reason: 'listed' })
    }
  }
  if (profile.mode === 'exclusive') {
    const listed = new Set(profile.mods.map((entry) => entry.key))
    for (const row of state.rows) {
      if (!listed.has(row.key) && isEnabled(row) && row.toggleBlock === null) {
        changes.push({
          modRef: row.modRef,
          key: row.key,
          name: row.name,
          enable: false,
          reason: 'unlisted',
        })
      }
    }
  }
  return changes
}

function fingerprintOf(changes: readonly DiffChange[]): string {
  return changes.map((change) => `${change.enable ? '+' : '-'}${change.key}\n`).join('')
}

export function diffProfile(state: MockState, id: string): ProfileDiff {
  const profile = profileById(state, id)
  const changes = changesFor(state, profile)
  const missing = profile.mods
    .filter((entry) => !state.rows.some((row) => row.key === entry.key))
    .map((entry) => entry.key)
  return {
    profileId: id,
    mode: profile.mode,
    changes,
    missing,
    skipped: [],
    unchanged: profile.mods.length - missing.length - changes.length,
    fingerprint: fingerprintOf(changes),
  }
}

// The diff must still be the one the user saw; a snapshot of the current state comes first.
export function applyProfile(state: MockState, request: ApplyRequest): ApplyOutcome {
  const profile = profileById(state, request.profileId)
  const changes = changesFor(state, profile)
  if (fingerprintOf(changes) !== request.fingerprint) fail({ kind: 'stale' })
  const now = new Date().toISOString()
  const snapshotId = nextId(state, 'snapshot')
  state.profiles.push(
    makeProfile(snapshotId, {
      name: '',
      mods: captureCurrent(state),
      created: now,
      updated: now,
      source: { kind: 'snapshot', of: profile.id },
    }),
  )
  for (const change of changes) {
    const row = state.rows.find((candidate) => candidate.key === change.key)
    if (row) setEnabled(row, change.enable)
  }
  profile.lastApplied = now
  state.lastApplied = { profileId: profile.id, at: now }
  return { snapshot: changed(state), changed: changes.length, snapshotId, skipped: [] }
}
