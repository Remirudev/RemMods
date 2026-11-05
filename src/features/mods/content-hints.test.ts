import { describe, expect, it } from 'vitest'
import { i18n } from '../../lib/i18n'
import { characterLabel, characterSummary, isRawCharacter, slotLabel } from './content-hints'

const t = i18n.getFixedT('en', 'mods')

describe('characterLabel', () => {
  it('shows the confirmed name', () => {
    expect(characterLabel({ code: 'ker', name: 'Kunimitsu', common: false }, t)).toBe('Kunimitsu')
  })

  it('shows the shared code as all characters', () => {
    expect(characterLabel({ code: 'cmn', name: null, common: true }, t)).toBe('All characters')
  })

  it('falls back to the raw code', () => {
    const hint = { code: 'zzz', name: null, common: false }
    expect(characterLabel(hint, t)).toBe('zzz')
    expect(isRawCharacter(hint)).toBe(true)
  })

  it('names the shared sets of women and men', () => {
    const women = { code: 'cf0', name: null, common: false }
    expect(characterLabel(women, t)).toBe('All women')
    expect(isRawCharacter(women)).toBe(false)
    expect(characterLabel({ code: 'cm0', name: null, common: false }, t)).toBe('All men')
  })
})

describe('slotLabel', () => {
  it('translates a known slot', () => {
    expect(slotLabel({ code: 'hir', slot: 'hair' }, t)).toBe('Hair')
    expect(slotLabel({ code: 'efc', slot: 'eyeMakeup' }, t)).toBe('Eye make-up')
  })

  it('falls back to the raw code', () => {
    expect(slotLabel({ code: 'ex0', slot: null }, t)).toBe('ex0')
  })
})

describe('characterSummary', () => {
  const hint = (name: string) => ({ code: name.toLowerCase(), name, common: false })

  it('lists a few names by alphabet', () => {
    expect(characterSummary([hint('Reina'), hint('Anna')], t, 'en')).toBe('Anna, Reina')
  })

  it('counts the rest', () => {
    const many = ['Reina', 'Anna', 'Jun', 'Nina', 'Lili'].map(hint)
    expect(characterSummary(many, t, 'en')).toBe('Anna, Jun, Lili +2')
  })

  it('is empty for a mod that names nobody', () => {
    expect(characterSummary([], t, 'en')).toBe('')
  })
})
