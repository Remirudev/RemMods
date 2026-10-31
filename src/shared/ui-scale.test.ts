import { describe, expect, it } from 'vitest'
import { clampUiScale, nextUiScale, uiScaleActionForKey } from './ui-scale'

const ctrl = { ctrlKey: true, altKey: false, metaKey: false }

describe('ui scale', () => {
  it('clamps to 80–150 % and rounds to 5 % steps', () => {
    expect(clampUiScale(0.5)).toBe(0.8)
    expect(clampUiScale(2)).toBe(1.5)
    expect(clampUiScale(1.07)).toBe(1.05)
    expect(clampUiScale(1.05)).toBe(1.05)
  })

  it('steps up, down and back to 100 %', () => {
    expect(nextUiScale(1, 'increase')).toBe(1.05)
    expect(nextUiScale(1, 'decrease')).toBe(0.95)
    expect(nextUiScale(1.5, 'increase')).toBe(1.5)
    expect(nextUiScale(1.3, 'reset')).toBe(1)
  })

  it('maps Ctrl + = / + / − / 0 and ignores AltGr', () => {
    expect(uiScaleActionForKey({ ...ctrl, key: '=' })).toBe('increase')
    expect(uiScaleActionForKey({ ...ctrl, key: '+' })).toBe('increase')
    expect(uiScaleActionForKey({ ...ctrl, key: '-' })).toBe('decrease')
    expect(uiScaleActionForKey({ ...ctrl, key: '0' })).toBe('reset')
    expect(uiScaleActionForKey({ ...ctrl, altKey: true, key: '0' })).toBeNull()
    expect(uiScaleActionForKey({ ...ctrl, ctrlKey: false, key: '0' })).toBeNull()
  })
})
