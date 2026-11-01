import { describe, expect, it } from 'vitest'
import { i18n } from '../../lib/i18n'
import { characterLabel, isRawCharacter, slotLabel } from './content-hints'

const t = i18n.getFixedT('en', 'mods')

describe('characterLabel', () => {
  it('shows the confirmed name', () => {
    expect(characterLabel({ code: 'ker', name: 'Kunimitsu', common: false }, t)).toBe('Kunimitsu')
  })

  it('shows the shared code as all characters', () => {
    expect(characterLabel({ code: 'cmn', name: null, common: true }, t)).toBe('All characters')
  })

  it('falls back to the raw code', () => {
    const hint = { code: 'zbr', name: null, common: false }
    expect(characterLabel(hint, t)).toBe('zbr')
    expect(isRawCharacter(hint)).toBe(true)
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
