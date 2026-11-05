import { describe, expect, it } from 'vitest'
import { makeModItem } from '../../test/fixtures'
import {
  charactersOf,
  chosenCharacter,
  coverPicture,
  internalName,
  itemGroups,
  listPicture,
  replacedNames,
} from './replaces-model'

const reina = { character: { code: 'zbr', name: 'Reina', common: false } }
const women = { character: { code: 'cf0', name: null, common: false } }

function keys(items: readonly { key: string }[]) {
  return items.map((item) => item.key)
}

describe('charactersOf', () => {
  it('lists each character once by name, the shared sets last', () => {
    const items = [
      makeModItem('zbr_bdu_a', reina),
      makeModItem('cf0_sho_a', women),
      makeModItem('ker_bdu_a'),
      makeModItem('ker_bdu_b'),
    ]
    expect(charactersOf(items, 'en').map((hint) => hint.code)).toEqual(['ker', 'zbr', 'cf0'])
  })
})

describe('chosenCharacter', () => {
  const characters = [
    { code: 'ker', name: 'Kunimitsu', common: false },
    { code: 'zbr', name: 'Reina', common: false },
  ]

  it('keeps the wanted one when the mod has it', () => {
    expect(chosenCharacter(characters, 'zbr')).toBe('zbr')
  })

  it('falls back to the first', () => {
    expect(chosenCharacter(characters, 'rat')).toBe('ker')
    expect(chosenCharacter(characters, null)).toBe('ker')
    expect(chosenCharacter([], 'ker')).toBeNull()
  })
})

describe('itemGroups', () => {
  it('shows one character, outfits first', () => {
    const items = [
      makeModItem('ker_acc_a', { slot: { code: 'acc', slot: 'accessory' } }),
      makeModItem('ker_har_a', { slot: { code: 'har', slot: 'hair' } }),
      makeModItem('zbr_bdf_a', reina),
      makeModItem('ker_bdf_a', { slot: { code: 'bdf', slot: 'fullBody' } }),
      makeModItem('ker_xyz_a', { slot: { code: 'xyz', slot: null } }),
    ]
    expect(keys(itemGroups(items, 'ker').main)).toEqual([
      'ker_bdf_a',
      'ker_har_a',
      'ker_acc_a',
      'ker_xyz_a',
    ])
  })

  it('moves pieces without a picture of their own aside', () => {
    const items = [
      makeModItem('ker_bdf_a'),
      makeModItem('ker_sho_hidden', { modPicture: null }),
      makeModItem('ker_btm_hidden', { modPicture: null }),
    ]
    const { main, other } = itemGroups(items, 'ker')
    expect(keys(main)).toEqual(['ker_bdf_a'])
    expect(keys(other)).toEqual(['ker_btm_hidden', 'ker_sho_hidden'])
  })

  it('keeps everything in view when the mod ships no pictures at all', () => {
    const items = [makeModItem('ker_bdf_a', { modPicture: null })]
    expect(keys(itemGroups(items, 'ker').main)).toEqual(['ker_bdf_a'])
    expect(itemGroups(items, 'ker').other).toEqual([])
  })
})

describe('internalName', () => {
  it('is the key without the character', () => {
    expect(internalName(makeModItem('ker_bdf_sfyor'))).toBe('bdf_sfyor')
  })
})

describe('coverPicture', () => {
  const outfit = { slot: { code: 'bdf', slot: 'fullBody' as const } }
  const hair = { slot: { code: 'har', slot: 'hair' as const } }

  it('prefers an outfit of the chosen character', () => {
    const items = [
      makeModItem('ker_har_a', hair),
      makeModItem('ker_bdf_a', outfit),
      makeModItem('zbr_bdf_a', { ...outfit, ...reina }),
    ]
    expect(coverPicture(items, 'ker')).toBe('mod-ker_bdf_a')
    expect(coverPicture(items, 'zbr')).toBe('mod-zbr_bdf_a')
  })

  it('takes any character’s picture when the chosen one has none', () => {
    const items = [makeModItem('zbr_bdf_a', { ...outfit, ...reina })]
    expect(coverPicture(items, 'ker')).toBe('mod-zbr_bdf_a')
  })

  it('never shows the game’s picture as the mod’s look', () => {
    expect(coverPicture([makeModItem('ker_bdf_a', { modPicture: null })], 'ker')).toBeNull()
  })
})

describe('listPicture', () => {
  const items = [makeModItem('ker_bdf_a'), makeModItem('zbr_bdf_a', reina)]

  it('takes the owner’s picture first', () => {
    expect(listPicture({ modRef: 'm', ownPicture: 'own', items }, 'zbr')).toBe('own')
  })

  it('else shows the mod’s look for the character', () => {
    expect(listPicture({ modRef: 'm', ownPicture: null, items }, 'zbr')).toBe('mod-zbr_bdf_a')
  })

  it('has nothing to show before the items arrive', () => {
    expect(listPicture(undefined, 'ker')).toBeNull()
  })
})

describe('replacedNames', () => {
  it('names what the chosen character gets, without hidden pieces, each once', () => {
    const items = [
      makeModItem('ker_bdf_a', { name: 'Outfit' }),
      makeModItem('ker_bdu_b', { name: 'Outfit' }),
      makeModItem('ker_sho_hidden', { name: 'Barefoot', modPicture: null }),
      makeModItem('zbr_bdf_a', { ...reina, name: 'Reina outfit' }),
    ]
    expect(replacedNames(items, 'ker', 'en')).toEqual(['Outfit'])
    expect(replacedNames(items, 'zbr', 'en')).toEqual(['Reina outfit'])
  })

  it('takes the internal name when there is no text', () => {
    expect(replacedNames([makeModItem('ker_bdf_yor')], 'ker', 'en')).toEqual(['bdf_yor'])
  })
})
