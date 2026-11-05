import { describe, expect, it } from 'vitest'
import {
  COLUMN_SIZES,
  clampColumnWidth,
  defaultColumnLayout,
  HIDEABLE_COLUMN_IDS,
  isModColumnId,
  keyboardWidth,
  parseColumnLayout,
  serializeColumnLayout,
} from './column-layout'

describe('column layout', () => {
  it('hides Version and Category by default and stores no width', () => {
    const layout = defaultColumnLayout()
    expect(layout.sizing).toEqual({})
    expect(Object.keys(layout.visibility).sort()).toEqual([...HIDEABLE_COLUMN_IDS].sort())
    const hidden = Object.entries(layout.visibility).filter(([, visible]) => !visible)
    expect(hidden.map(([id]) => id).sort()).toEqual(['category', 'version'])
  })

  it('round-trips widths and hidden columns', () => {
    const layout = {
      sizing: { name: 420, issues: 90 },
      visibility: { ...defaultColumnLayout().visibility, kind: false, version: true },
    }
    const raw = serializeColumnLayout(layout)
    expect(JSON.parse(raw)).toEqual({
      widths: { name: 420, issues: 90 },
      hidden: ['kind', 'category'],
    })
    expect(parseColumnLayout(raw)).toEqual(layout)
  })

  it('falls back to the defaults for nothing stored or no layout at all', () => {
    for (const raw of [null, '', 'not json', '[]', '"text"', '42', 'null']) {
      expect(parseColumnLayout(raw), String(raw)).toEqual(defaultColumnLayout())
    }
  })

  it('drops unknown columns and invalid widths one by one', () => {
    const raw = JSON.stringify({
      widths: {
        name: 300.4,
        state: 5,
        kind: 'wide',
        size: Number.MAX_SAFE_INTEGER,
        issues: null,
        toggle: 200,
        bogus: 150,
      },
      hidden: ['state', 'name', 'menu', 'bogus', 7],
    })
    const layout = parseColumnLayout(raw)
    expect(layout.sizing).toEqual({ name: 300 })
    const hidden = Object.entries(layout.visibility).filter(([, visible]) => !visible)
    expect(hidden.map(([id]) => id)).toEqual(['state'])
  })

  it('keeps the default columns hidden when the hidden list is not a list', () => {
    const layout = parseColumnLayout(JSON.stringify({ widths: { name: 500 }, hidden: 'kind' }))
    expect(layout).toEqual({ ...defaultColumnLayout(), sizing: { name: 500 } })
  })

  it('clamps widths to each column’s limits', () => {
    const { minSize, maxSize } = COLUMN_SIZES.state
    expect(clampColumnWidth('state', 1)).toBe(minSize)
    expect(clampColumnWidth('state', 99_999)).toBe(maxSize)
    expect(clampColumnWidth('state', 150.6)).toBe(151)
  })

  it('steps widths with the arrow keys, further with Shift', () => {
    expect(keyboardWidth(100, 'ArrowRight', false)).toBe(110)
    expect(keyboardWidth(100, 'ArrowLeft', false)).toBe(90)
    expect(keyboardWidth(100, 'ArrowRight', true)).toBe(150)
    expect(keyboardWidth(100, 'Enter', false)).toBeNull()
  })

  it('knows its column ids', () => {
    expect(isModColumnId('contentConflicts')).toBe(true)
    expect(isModColumnId('toggle')).toBe(false)
    expect(isModColumnId('menu')).toBe(false)
  })
})
