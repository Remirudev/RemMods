import { describe, expect, it } from 'vitest'
import { i18n } from '../../lib/i18n'
import type { AssetEntry } from '../../lib/ipc/types'
import { filterEntries, originLabelKey, sharedLabel } from './content-entries'

const t = i18n.getFixedT('en', 'mods')

function entry(path: string, overrides: Partial<AssetEntry> = {}): AssetEntry {
  return {
    path,
    container: 'a.utoc',
    live: true,
    origin: 'adds',
    character: null,
    slot: null,
    sharedWith: [],
    ...overrides,
  }
}

const ENTRIES: AssetEntry[] = [
  entry('Character/Item/Hair/BI_Hair_Kunimitsu.uasset', {
    character: { code: 'ker', name: 'Kunimitsu', common: false },
    slot: { code: 'hir', slot: 'hair' },
  }),
  entry('Materials/Shared.uasset'),
]

describe('filterEntries', () => {
  it('returns everything for a blank needle', () => {
    expect(filterEntries(ENTRIES, '  ', t, 'en')).toHaveLength(2)
  })

  it('matches the path case-insensitively', () => {
    expect(filterEntries(ENTRIES, 'MATERIALS', t, 'en').map((e) => e.path)).toEqual([
      'Materials/Shared.uasset',
    ])
  })

  it('matches the character label and code', () => {
    expect(filterEntries(ENTRIES, 'kunimitsu', t, 'en')).toHaveLength(1)
    expect(filterEntries(ENTRIES, 'ker', t, 'en')).toHaveLength(1)
  })

  it('matches the translated slot label', () => {
    expect(filterEntries(ENTRIES, 'hair', t, 'en')).toHaveLength(1)
    expect(filterEntries(ENTRIES, 'nothing', t, 'en')).toHaveLength(0)
  })
})

describe('originLabelKey', () => {
  it('maps each origin to its text key', () => {
    expect(originLabelKey('adds')).toBe('adds')
    expect(originLabelKey('replaces')).toBe('replaces')
    expect(originLabelKey('unknown')).toBe('unknown')
  })
})

describe('sharedLabel', () => {
  it('names what a differing shared file does', () => {
    expect(sharedLabel('replaces', 0, t)).toBeNull()
    expect(sharedLabel('replaces', 1, t)).toBe('Both replace this game file')
    expect(sharedLabel('unknown', 2, t)).toBe('Different versions in 2 other mods')
  })
})
