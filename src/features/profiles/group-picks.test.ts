import { describe, expect, it } from 'vitest'
import { makeModRow } from '../../test/fixtures'
import { makeProfile } from '../../test/profile-fixtures'
import {
  countPicks,
  isRowOn,
  matchesPickSearch,
  pickEntries,
  pickItems,
  picksFromProfile,
  picksFromRows,
  withPicks,
} from './group-picks'

const ON = makeModRow('On')
const PARTIAL = makeModRow('Partial', { state: 'partial' })
const OFF = makeModRow('Off', { state: 'disabled' })
const LOADED = makeModRow('Loaded', {
  state: 'disabled',
  logicPrefixed: true,
  issues: [{ kind: 'logicDisabledButLoaded' }],
})
const EMPTY = makeModRow('Empty', { state: 'noPakFiles', toggleBlock: 'noPakFiles' })
const TWIN = makeModRow('Twin', { ambiguous: true })

describe('isRowOn', () => {
  it('counts enabled, partial and still-loaded LogicMods as on, like the core', () => {
    expect([ON, PARTIAL, LOADED].map(isRowOn)).toEqual([true, true, true])
    expect([OFF, EMPTY].map(isRowOn)).toEqual([false, false])
  })
})

describe('picksFromRows', () => {
  it('picks what a saved state would hold and leaves ambiguous or blocked rows alone', () => {
    const picks = picksFromRows([ON, PARTIAL, OFF, LOADED, EMPTY, TWIN])

    expect(Object.fromEntries(picks)).toEqual({
      on: 'on',
      partial: 'on',
      off: 'off',
      loaded: 'on',
    })
  })
})

describe('picksFromProfile', () => {
  it('maps the stored entries, and nothing for a new group', () => {
    const profile = makeProfile('p1', {
      mods: [
        { key: 'a', state: 'enabled' },
        { key: 'b', state: 'disabled' },
      ],
    })

    expect(Object.fromEntries(picksFromProfile(profile))).toEqual({ a: 'on', b: 'off' })
    expect(picksFromProfile(null).size).toBe(0)
  })
})

describe('withPicks, countPicks and pickEntries', () => {
  it('sets several keys at once, and leave alone removes them from the list', () => {
    const start = new Map([['c', 'off' as const]])
    const picked = withPicks(start, ['b', 'a'], 'on')
    const skipped = withPicks(picked, ['c'], 'skip')

    expect(start.get('c')).toBe('off')
    expect(countPicks(picked)).toEqual({ on: 2, off: 1 })
    expect(countPicks(skipped)).toEqual({ on: 2, off: 0 })
    expect(pickEntries(picked)).toEqual([
      { key: 'a', state: 'enabled' },
      { key: 'b', state: 'enabled' },
      { key: 'c', state: 'disabled' },
    ])
  })
})

describe('pickItems', () => {
  it('lists installed mods by name, then listed mods that are not installed', () => {
    const rows = [makeModRow('Mod 10'), makeModRow('mod 9', { state: 'disabled' }), TWIN, TWIN]
    const items = pickItems(rows, ['mod 9', 'Standard:Gone'], 'en')

    expect(items.map((item) => [item.name, item.on])).toEqual([
      ['mod 9', false],
      ['Mod 10', true],
      ['Twin', true],
      ['Gone', null],
    ])
  })
})

describe('matchesPickSearch', () => {
  it('matches the name or the key, ignoring case', () => {
    const item = { key: 'Standard:FineMods/Apron', name: 'Apron', on: true }

    expect(matchesPickSearch(item, '', 'en')).toBe(true)
    expect(matchesPickSearch(item, 'apr', 'en')).toBe(true)
    expect(matchesPickSearch(item, 'finemods', 'en')).toBe(true)
    expect(matchesPickSearch(item, 'shell', 'en')).toBe(false)
  })
})
