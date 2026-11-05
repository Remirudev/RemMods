import { describe, expect, it } from 'vitest'
import type { DiffChange, ProfileDiff } from '../../lib/ipc/types'
import { groupDiff, keyName } from './diff-groups'

function change(key: string, enable: boolean, reason: DiffChange['reason'] = 'listed'): DiffChange {
  return { modRef: `m:${key}`, key, name: key, enable, reason }
}

const DIFF: ProfileDiff = {
  profileId: 'p',
  mode: 'exclusive',
  changes: [change('a', true), change('b', false, 'unlisted'), change('c', false)],
  missing: ['Standard:Gone'],
  skipped: [{ key: 'd', name: 'D', wanted: 'enabled', reason: 'ambiguous' }],
  unchanged: 4,
  fingerprint: '+a\n-b\n-c\n',
}

describe('groupDiff', () => {
  it('splits the changes by direction and keeps the order', () => {
    const groups = groupDiff(DIFF)
    expect(groups.enable.map((entry) => entry.key)).toEqual(['a'])
    expect(groups.disable.map((entry) => entry.key)).toEqual(['b', 'c'])
    expect(groups.disable[0]?.reason).toBe('unlisted')
    expect(groups.skipped).toHaveLength(1)
    expect(groups.missing).toEqual(['Standard:Gone'])
    expect(groups.hasChanges).toBe(true)
  })

  it('has no changes when only skips and missing mods remain', () => {
    expect(groupDiff({ ...DIFF, changes: [] }).hasChanges).toBe(false)
  })
})

describe('keyName', () => {
  it('cuts the kind before the first colon', () => {
    expect(keyName('Standard:Cool: Skin')).toBe('Cool: Skin')
    expect(keyName('plain')).toBe('plain')
  })
})
