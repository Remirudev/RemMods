import { describe, expect, it } from 'vitest'
import type { Profile, ProfileSummary } from '../../lib/ipc/types'
import { profileTitle, relativeTime, snapshotTarget, splitProfiles } from './profile-label'

function profile(id: string, overrides: Partial<Profile> = {}): Profile {
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

function summary(p: Profile): ProfileSummary {
  return { profile: p, enabled: 0, disabled: 0, missing: 0, matchesCurrent: true }
}

describe('profileTitle', () => {
  it('shows the name, or the translated word for a blank one', () => {
    expect(profileTitle(profile('a', { name: 'Tournament' }), 'Unnamed')).toBe('Tournament')
    expect(profileTitle(profile('a', { name: '  ' }), 'Unnamed')).toBe('Unnamed')
  })
})

describe('snapshotTarget', () => {
  const user = profile('p1', { name: 'Main' })
  const snapshot = profile('s1', { name: '', source: { kind: 'snapshot', of: 'p1' } })

  it('finds the profile the snapshot was taken for', () => {
    expect(snapshotTarget(snapshot, [user, snapshot])?.name).toBe('Main')
  })

  it('is null once that profile is deleted, and for a non-snapshot', () => {
    expect(snapshotTarget(snapshot, [snapshot])).toBeNull()
    expect(snapshotTarget(user, [user])).toBeNull()
  })
})

describe('splitProfiles', () => {
  it('sorts user profiles by name and snapshots newest first', () => {
    const old = profile('s1', {
      source: { kind: 'snapshot', of: 'x' },
      created: '2026-10-01T00:00:00.000Z',
    })
    const recent = profile('s2', {
      source: { kind: 'snapshot', of: 'x' },
      created: '2026-11-01T00:00:00.000Z',
    })
    const { user, snapshots } = splitProfiles(
      [old, profile('beta'), recent, profile('Alpha')].map(summary),
      'en',
    )
    expect(user.map((entry) => entry.profile.name)).toEqual(['Alpha', 'beta'])
    expect(snapshots.map((entry) => entry.profile.id)).toEqual(['s2', 's1'])
  })
})

describe('relativeTime', () => {
  const now = Date.parse('2026-11-01T12:00:00.000Z')

  it('picks days, hours or minutes', () => {
    expect(relativeTime('2026-10-30T12:00:00.000Z', 'en', now)).toBe('2 days ago')
    expect(relativeTime('2026-11-01T09:00:00.000Z', 'en', now)).toBe('3 hours ago')
    expect(relativeTime('2026-11-01T11:55:00.000Z', 'en', now)).toBe('5 minutes ago')
  })

  it('falls back to the raw text for a bad stamp', () => {
    expect(relativeTime('nope', 'en', now)).toBe('nope')
  })
})
