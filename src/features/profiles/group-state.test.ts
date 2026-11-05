import { describe, expect, it } from 'vitest'
import { makeModRow } from '../../test/fixtures'
import { makeProfile, makeSummary, makeView } from '../../test/profile-fixtures'
import { groupClashes, groupInitials, groupItems, isLit, onKeysOf } from './group-state'

const SHARED_PAK = makeModRow('Apron', {
  conflictsWith: [{ modRef: 'm:mavis', key: 'mavis', name: 'Mavis', paks: ['x.pak'] }],
})
const MAVIS = makeModRow('Mavis', {
  conflictsWith: [{ modRef: 'm:apron', key: 'apron', name: 'Apron', paks: ['x.pak'] }],
})
const OUTFIT = makeModRow('Police', {
  contentConflicts: [
    {
      modRef: 'm:ghislaine',
      key: 'ghislaine',
      name: 'Ghislaine',
      shared: 3,
      outfit: 2,
      sample: [],
    },
  ],
})
const SHOES_ONLY = makeModRow('Shoes', {
  contentConflicts: [
    {
      modRef: 'm:ghislaine',
      key: 'ghislaine',
      name: 'Ghislaine',
      shared: 6,
      outfit: 0,
      sample: [],
    },
  ],
})
const GHISLAINE = makeModRow('Ghislaine')

describe('onKeysOf', () => {
  it('lists the keys a group switches on', () => {
    const profile = makeProfile('p1', {
      mods: [
        { key: 'a', state: 'enabled' },
        { key: 'b', state: 'disabled' },
        { key: 'c', state: 'enabled' },
      ],
    })

    expect(onKeysOf(profile)).toEqual(['a', 'c'])
  })
})

describe('isLit', () => {
  it('is on while the mods match, but never for an empty additive group', () => {
    const listed = makeProfile('p1', { mods: [{ key: 'a', state: 'enabled' }] })
    const empty = makeProfile('p2')
    const allOff = makeProfile('p3', { mode: 'exclusive' })

    expect(isLit(makeSummary(listed))).toBe(true)
    expect(isLit(makeSummary(listed, { matchesCurrent: false }))).toBe(false)
    expect(isLit(makeSummary(empty))).toBe(false)
    expect(isLit(makeSummary(allOff))).toBe(true)
  })
})

describe('groupClashes', () => {
  const rows = [SHARED_PAK, MAVIS, OUTFIT, SHOES_ONLY, GHISLAINE]

  it('reports each clashing pair inside the group once, by file and by outfit', () => {
    const clashes = groupClashes(rows, ['apron', 'mavis', 'police', 'ghislaine'])

    expect(clashes).toEqual([
      { first: 'Apron', second: 'Mavis' },
      { first: 'Ghislaine', second: 'Police' },
    ])
  })

  it('ignores clashes with mods outside the group and shared shoes or bottoms', () => {
    expect(groupClashes(rows, ['apron', 'police'])).toEqual([])
    expect(groupClashes(rows, ['shoes', 'ghislaine'])).toEqual([])
  })
})

describe('groupItems', () => {
  it('sorts user groups by name, leaves snapshots out and marks the ones that are on', () => {
    const fine = makeProfile('p1', { name: 'FineMods', mods: [{ key: 'apron', state: 'enabled' }] })
    const blender = makeProfile('p2', {
      name: 'BlenderSphere',
      mods: [
        { key: 'mavis', state: 'enabled' },
        { key: 'apron', state: 'disabled' },
      ],
    })
    const snapshot = makeProfile('s1', { name: '', source: { kind: 'snapshot', of: 'p1' } })
    const view = makeView([
      makeSummary(fine),
      makeSummary(blender, { matchesCurrent: false }),
      makeSummary(snapshot),
    ])

    const items = groupItems(view, [SHARED_PAK, MAVIS], 'en', 'Unnamed')

    expect(items.map((item) => [item.title, item.lit, item.onKeys])).toEqual([
      ['BlenderSphere', false, ['mavis']],
      ['FineMods', true, ['apron']],
    ])
  })
})

describe('groupInitials', () => {
  it('takes the first letters of two words, or the first two letters of one', () => {
    expect(groupInitials('Fine Mods')).toBe('FM')
    expect(groupInitials('blendersphere')).toBe('BL')
    expect(groupInitials('  ')).toBe('')
  })
})
