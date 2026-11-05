import type {
  DiffChange,
  Profile,
  ProfileDiff,
  ProfileSummary,
  ProfilesView,
} from '../lib/ipc/types'

export function makeProfile(id: string, overrides: Partial<Profile> = {}): Profile {
  return {
    id,
    name: id,
    description: '',
    mode: 'additive',
    mods: [],
    created: '2026-11-01T10:00:00.000Z',
    updated: '2026-11-01T10:00:00.000Z',
    lastApplied: null,
    source: { kind: 'user' },
    skinSet: null,
    ...overrides,
  }
}

export function makeSummary(
  profile: Profile,
  overrides: Partial<ProfileSummary> = {},
): ProfileSummary {
  return { profile, enabled: 2, disabled: 1, missing: 0, matchesCurrent: true, ...overrides }
}

export function makeView(
  summaries: ProfileSummary[],
  overrides: Partial<ProfilesView> = {},
): ProfilesView {
  return { profiles: summaries, lastApplied: null, readOnly: false, problem: null, ...overrides }
}

export function makeChange(key: string, enable: boolean): DiffChange {
  return { modRef: `m:${key}`, key, name: key, enable, reason: 'listed' }
}

export function makeDiff(overrides: Partial<ProfileDiff> = {}): ProfileDiff {
  return {
    profileId: 'p1',
    mode: 'additive',
    changes: [makeChange('alpha', true), makeChange('beta', false)],
    missing: [],
    skipped: [],
    unchanged: 0,
    fingerprint: '+alpha\n-beta\n',
    ...overrides,
  }
}
