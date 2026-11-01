import { describe, expect, it } from 'vitest'
import { makeModRow } from '../../test/fixtures'
import { compareMods, modCollator } from './compare-mods'

const collator = modCollator('en')

describe('compareMods', () => {
  it('sorts numbers inside names numerically', () => {
    const two = makeModRow('Mod 2')
    const ten = makeModRow('Mod 10')
    expect(compareMods(two, ten, 'name', collator)).toBeLessThan(0)
    expect(compareMods(ten, two, 'name', collator)).toBeGreaterThan(0)
  })

  it('orders states enabled < partial < disabledLoaded < disabled < noPakFiles < unknown', () => {
    const loadedWhileDisabled = makeModRow('d', {
      state: 'disabled',
      logicPrefixed: true,
      payload: { livePak: 1, offPak: 0, liveCompanion: 0, offCompanion: 0 },
    })
    const ordered = [
      makeModRow('a', { state: 'enabled' }),
      makeModRow('b', { state: 'partial' }),
      loadedWhileDisabled,
      makeModRow('e', { state: 'disabled' }),
      makeModRow('f', { state: 'noPakFiles' }),
      makeModRow('g', { state: 'unknown' }),
    ]
    const shuffled = [ordered[4], ordered[0], ordered[5], ordered[2], ordered[1], ordered[3]]
    const sorted = [...shuffled].sort((a, b) => compareMods(a!, b!, 'state', collator))
    expect(sorted.map((row) => row?.name)).toEqual(ordered.map((row) => row.name))
  })

  it('orders kinds standard < legacy < logic < loose', () => {
    const kinds = ['loose', 'logic', 'legacy', 'standard'] as const
    const sorted = kinds
      .map((kind) => makeModRow(kind, { kind }))
      .sort((a, b) => compareMods(a, b, 'kind', collator))
    expect(sorted.map((row) => row.kind)).toEqual(['standard', 'legacy', 'logic', 'loose'])
  })

  it('treats an unknown size as smaller than any size', () => {
    const unknown = makeModRow('a', { sizeBytes: null })
    const empty = makeModRow('b', { sizeBytes: 0 })
    expect(compareMods(unknown, empty, 'size', collator)).toBeLessThan(0)
  })

  it('compares versions and categories by their raw values', () => {
    const base = makeModRow('x').info
    const low = makeModRow('a', { info: { ...base, version: '1.2', category: 'Apple' } })
    const high = makeModRow('b', { info: { ...base, version: '1.10', category: 'Banana' } })
    expect(compareMods(low, high, 'version', collator)).toBeLessThan(0)
    expect(compareMods(low, high, 'category', collator)).toBeLessThan(0)
  })

  it('counts conflicts and issues', () => {
    const link = { modRef: 'm:x', key: 'x', name: 'X', paks: ['x.pak'] }
    const none = makeModRow('a')
    const some = makeModRow('b', {
      conflictsWith: [link, link],
      issues: [{ kind: 'linkedMod' }],
    })
    expect(compareMods(none, some, 'conflicts', collator)).toBeLessThan(0)
    expect(compareMods(none, some, 'issues', collator)).toBeLessThan(0)
  })

  it('breaks ties by name and then by ModRef', () => {
    const a = makeModRow('Same', { modRef: 'm:1' })
    const b = makeModRow('Same', { modRef: 'm:2' })
    expect(compareMods(a, b, 'kind', collator)).toBeLessThan(0)
    expect(compareMods(b, a, 'kind', collator)).toBeGreaterThan(0)
    expect(compareMods(a, a, 'kind', collator)).toBe(0)
    const early = makeModRow('Alpha', { modRef: 'm:9' })
    expect(compareMods(early, b, 'kind', collator)).toBeLessThan(0)
  })
})
