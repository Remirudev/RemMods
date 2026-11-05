import { describe, expect, it } from 'vitest'
import type { ModKind, ModRow } from '../../lib/ipc/types'
import { makeModRow } from '../../test/fixtures'
import { characterHints, EMPTY_FILTER, filterMods, isFilterActive } from './filter-mods'
import type { ModFilter } from './types'

const KIND_LABELS: Record<ModKind, string> = {
  standard: 'Mods',
  legacy: '~mods',
  logic: 'LogicMods',
  loose: 'Loose',
}
const kindLabel = (kind: ModKind) => KIND_LABELS[kind]
const info = makeModRow('x').info

const ROWS: ModRow[] = [
  makeModRow('Alpha Skin', { info: { ...info, category: 'Costume', author: 'Rem' } }),
  makeModRow('Beta', { state: 'partial', kind: 'legacy', key: 'beta-key' }),
  makeModRow('Gamma', { state: 'disabled' }),
  makeModRow('Delta', { state: 'noPakFiles' }),
  makeModRow('Epsilon', { state: 'unknown' }),
  makeModRow('Logic Loaded', {
    state: 'disabled',
    kind: 'logic',
    logicPrefixed: true,
    payload: { livePak: 1, offPak: 0, liveCompanion: 0, offCompanion: 0 },
  }),
  makeModRow('Conflicted', {
    conflictsWith: [{ modRef: 'm:x', key: 'x', name: 'X', paks: ['a.pak'] }],
  }),
  makeModRow('Troubled', { issues: [{ kind: 'linkedMod' }] }),
  makeModRow('Same Stuff', {
    contentConflicts: [
      { modRef: 'm:x', key: 'x', name: 'X', shared: 2, outfit: 1, sample: ['a/b.uasset'] },
    ],
  }),
  makeModRow('Shared Shoes', {
    contentConflicts: [
      { modRef: 'm:y', key: 'y', name: 'Y', shared: 6, outfit: 0, sample: ['a/sho.uasset'] },
    ],
  }),
  makeModRow('Samurai', {
    content: {
      ...makeModRow('x').content,
      characters: [
        { code: 'ker', name: 'Kunimitsu', common: false },
        { code: 'zbr', name: null, common: false },
      ],
    },
  }),
]

function names(filter: Partial<ModFilter>, locale = 'en'): string[] {
  return filterMods(ROWS, { ...EMPTY_FILTER, ...filter }, kindLabel, locale).map((row) => row.name)
}

describe('filterMods', () => {
  it('returns every row for the empty filter', () => {
    expect(names({})).toHaveLength(ROWS.length)
  })

  it('matches the text in name, key, category, author and kind label', () => {
    expect(names({ text: 'alpha' })).toEqual(['Alpha Skin'])
    expect(names({ text: 'beta-key' })).toEqual(['Beta'])
    expect(names({ text: 'costume' })).toEqual(['Alpha Skin'])
    expect(names({ text: ' REM ' })).toEqual(['Alpha Skin'])
    expect(names({ text: '~mods' })).toEqual(['Beta'])
    expect(names({ text: 'logicmods' })).toEqual(['Logic Loaded'])
  })

  it('lowercases with the given locale', () => {
    const rows = [makeModRow('Istanbul'), makeModRow('Izmir')]
    const filter = { ...EMPTY_FILTER, text: 'ıstanbul' }
    expect(filterMods(rows, filter, kindLabel, 'tr').map((row) => row.name)).toEqual(['Istanbul'])
    expect(filterMods(rows, filter, kindLabel, 'en')).toEqual([])
  })

  it('filters by every state, including noPakFiles and unknown', () => {
    expect(names({ state: 'enabled' })).toContain('Alpha Skin')
    expect(names({ state: 'partial' })).toEqual(['Beta'])
    expect(names({ state: 'noPakFiles' })).toEqual(['Delta'])
    expect(names({ state: 'unknown' })).toEqual(['Epsilon'])
  })

  it('keeps a prefixed folder with live paks out of the disabled filter', () => {
    expect(names({ state: 'disabled' })).toEqual(['Gamma'])
    expect(names({ state: 'disabledLoaded' })).toEqual(['Logic Loaded'])
  })

  it('filters by kinds, conflicts and issues', () => {
    expect(names({ kinds: ['legacy', 'logic'] })).toEqual(['Beta', 'Logic Loaded'])
    expect(names({ conflictsOnly: true })).toEqual(['Conflicted'])
    expect(names({ issuesOnly: true })).toEqual(['Troubled'])
  })

  it('filters by content conflicts over upper-body or full-body clothing only', () => {
    expect(names({ contentConflictsOnly: true })).toEqual(['Same Stuff'])
  })

  it('searches the character names and codes of the contents', () => {
    expect(names({ text: 'Kunimitsu' })).toEqual(['Samurai'])
    expect(names({ text: 'ker' })).toEqual(['Samurai'])
    expect(names({ text: 'ZBR' })).toEqual(['Samurai'])
  })

  it('combines the criteria', () => {
    expect(names({ text: 'con', conflictsOnly: true })).toEqual(['Conflicted'])
    expect(names({ text: 'zzz' })).toEqual([])
  })

  it('keeps only the rows of a group picked in the sidebar, with the other criteria', () => {
    const inGroup = (keys: readonly string[] | null, filter: Partial<ModFilter> = {}) =>
      filterMods(ROWS, { ...EMPTY_FILTER, ...filter }, kindLabel, 'en', keys).map((row) => row.name)

    expect(inGroup(['beta-key', 'gamma', 'missing'])).toEqual(['Beta', 'Gamma'])
    expect(inGroup(['beta-key', 'gamma'], { state: 'disabled' })).toEqual(['Gamma'])
    expect(inGroup([])).toEqual([])
    expect(inGroup(null)).toHaveLength(ROWS.length)
  })
})

describe('isFilterActive', () => {
  it('is false for the empty filter and for whitespace only', () => {
    expect(isFilterActive(EMPTY_FILTER)).toBe(false)
    expect(isFilterActive({ ...EMPTY_FILTER, text: '   ' })).toBe(false)
  })

  it('is true for any criterion', () => {
    expect(isFilterActive({ ...EMPTY_FILTER, text: 'a' })).toBe(true)
    expect(isFilterActive({ ...EMPTY_FILTER, state: 'enabled' })).toBe(true)
    expect(isFilterActive({ ...EMPTY_FILTER, kinds: ['logic'] })).toBe(true)
    expect(isFilterActive({ ...EMPTY_FILTER, conflictsOnly: true })).toBe(true)
    expect(isFilterActive({ ...EMPTY_FILTER, contentConflictsOnly: true })).toBe(true)
    expect(isFilterActive({ ...EMPTY_FILTER, issuesOnly: true })).toBe(true)
  })
})

describe('the character filter', () => {
  const kuni = { code: 'ker', name: 'Kunimitsu', common: false }
  const reina = { code: 'zbr', name: 'Reina', common: false }
  const everyone = { code: 'cmn', name: null, common: true }
  const content = makeModRow('x').content
  const rows = [
    makeModRow('Kuni', { content: { ...content, characters: [kuni] } }),
    makeModRow('Both', { content: { ...content, characters: [kuni, reina] } }),
    makeModRow('Shared', { content: { ...content, characters: [everyone] } }),
    makeModRow('None'),
  ]
  const names = (characters: string[]) =>
    filterMods(rows, { ...EMPTY_FILTER, characters }, kindLabel, 'en').map((row) => row.name)

  it('keeps mods for the chosen characters and mods for everyone', () => {
    expect(names(['zbr'])).toEqual(['Both', 'Shared'])
    expect(names(['ker', 'zbr'])).toEqual(['Kuni', 'Both', 'Shared'])
  })

  it('keeps everything while nothing is chosen', () => {
    expect(names([])).toHaveLength(4)
  })

  it('counts as an active filter', () => {
    expect(isFilterActive({ ...EMPTY_FILTER, characters: ['ker'] })).toBe(true)
  })

  it('offers each character once and no "everyone" entry', () => {
    expect(characterHints(rows).map((hint) => hint.code)).toEqual(['ker', 'zbr'])
  })
})
