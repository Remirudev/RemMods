import { describe, expect, it } from 'vitest'
import { applyClick, extendTo, pruneSelection } from './selection'

const IDS = ['a', 'b', 'c', 'd', 'e']
const PLAIN = { shift: false, ctrl: false }

describe('applyClick', () => {
  it('selects one row and makes it the anchor', () => {
    expect(applyClick(['a', 'b'], 'a', IDS, 'c', PLAIN)).toEqual({
      selection: ['c'],
      anchor: 'c',
    })
  })

  it('toggles membership with ctrl and moves the anchor', () => {
    const ctrl = { shift: false, ctrl: true }
    expect(applyClick(['a'], 'a', IDS, 'c', ctrl)).toEqual({ selection: ['a', 'c'], anchor: 'c' })
    expect(applyClick(['a', 'c'], 'c', IDS, 'a', ctrl)).toEqual({ selection: ['c'], anchor: 'a' })
  })

  it('selects the range from the anchor with shift, in either direction, and keeps the anchor', () => {
    const shift = { shift: true, ctrl: false }
    expect(applyClick(['b'], 'b', IDS, 'd', shift)).toEqual({
      selection: ['b', 'c', 'd'],
      anchor: 'b',
    })
    expect(applyClick(['d'], 'd', IDS, 'b', shift)).toEqual({
      selection: ['b', 'c', 'd'],
      anchor: 'd',
    })
  })

  it('adds the range to the selection with ctrl+shift', () => {
    const both = { shift: true, ctrl: true }
    expect(applyClick(['a', 'd'], 'd', IDS, 'e', both).selection).toEqual(['a', 'd', 'e'])
  })

  it('falls back to a single selection when the anchor is gone or unset', () => {
    const shift = { shift: true, ctrl: false }
    expect(applyClick([], null, IDS, 'c', shift)).toEqual({ selection: ['c'], anchor: 'c' })
    expect(applyClick(['x'], 'x', IDS, 'c', shift)).toEqual({ selection: ['c'], anchor: 'c' })
  })
})

describe('extendTo', () => {
  it('grows and shrinks the range around the anchor', () => {
    expect(extendTo(['b'], 'b', IDS, 'c')).toEqual({ selection: ['b', 'c'], anchor: 'b' })
    expect(extendTo(['b', 'c', 'd'], 'b', IDS, 'a')).toEqual({
      selection: ['a', 'b'],
      anchor: 'b',
    })
  })

  it('starts at the target row without an anchor', () => {
    expect(extendTo([], null, IDS, 'c')).toEqual({ selection: ['c'], anchor: 'c' })
  })
})

describe('pruneSelection', () => {
  it('drops rows that are gone and keeps the order', () => {
    expect(pruneSelection(['e', 'a', 'z'], IDS)).toEqual(['e', 'a'])
  })

  it('returns the same array when nothing changed', () => {
    const selection = ['a', 'b']
    expect(pruneSelection(selection, IDS)).toBe(selection)
  })
})
