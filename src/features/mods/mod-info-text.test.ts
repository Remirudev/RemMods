import { describe, expect, it } from 'vitest'
import { i18n } from '../../lib/i18n'
import { TEST_INFO } from '../../test/fixtures'
import { modInfoText } from './mod-info-text'

const t = i18n.getFixedT('en', 'mods')

describe('modInfoText', () => {
  it('shows the translated placeholder for a defaulted category and author', () => {
    expect(modInfoText(TEST_INFO, 'category', t)).toBe('Misc')
    expect(modInfoText(TEST_INFO, 'author', t)).toBe('Unknown')
  })

  it('keeps the raw version and description even when defaulted', () => {
    expect(modInfoText(TEST_INFO, 'version', t)).toBe('1.0')
    expect(modInfoText(TEST_INFO, 'description', t)).toBe('')
  })

  it('shows the raw value when the file provided it', () => {
    const info = {
      ...TEST_INFO,
      category: 'Costume',
      defaulted: { ...TEST_INFO.defaulted, category: false },
    }
    expect(modInfoText(info, 'category', t)).toBe('Costume')
  })
})
