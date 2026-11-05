import { describe, expect, it } from 'vitest'
import { isNavigationKey, nextActiveIndex, tableKeyIntent } from './table-keyboard'

describe('nextActiveIndex', () => {
  it('starts at the first row, or the last for End, while none is active', () => {
    expect(nextActiveIndex(-1, 'ArrowDown', 10, 5)).toBe(0)
    expect(nextActiveIndex(-1, 'ArrowUp', 10, 5)).toBe(0)
    expect(nextActiveIndex(-1, 'End', 10, 5)).toBe(9)
  })

  it('moves by one and stops at both ends', () => {
    expect(nextActiveIndex(3, 'ArrowDown', 10, 5)).toBe(4)
    expect(nextActiveIndex(3, 'ArrowUp', 10, 5)).toBe(2)
    expect(nextActiveIndex(9, 'ArrowDown', 10, 5)).toBe(9)
    expect(nextActiveIndex(0, 'ArrowUp', 10, 5)).toBe(0)
  })

  it('jumps with Home, End and the page keys', () => {
    expect(nextActiveIndex(4, 'Home', 10, 5)).toBe(0)
    expect(nextActiveIndex(4, 'End', 10, 5)).toBe(9)
    expect(nextActiveIndex(1, 'PageDown', 10, 5)).toBe(6)
    expect(nextActiveIndex(8, 'PageDown', 10, 5)).toBe(9)
    expect(nextActiveIndex(6, 'PageUp', 10, 5)).toBe(1)
    expect(nextActiveIndex(2, 'PageUp', 10, 5)).toBe(0)
  })

  it('moves at least one row for a tiny page size', () => {
    expect(nextActiveIndex(2, 'PageDown', 10, 0)).toBe(3)
  })

  it('treats an index past the end (after filtering) as the last row', () => {
    expect(nextActiveIndex(20, 'ArrowUp', 5, 3)).toBe(3)
    expect(nextActiveIndex(20, 'ArrowDown', 5, 3)).toBe(4)
  })

  it('ignores other keys, including Space, and empty lists', () => {
    expect(nextActiveIndex(1, ' ', 10, 5)).toBeNull()
    expect(nextActiveIndex(1, 'Enter', 10, 5)).toBeNull()
    expect(nextActiveIndex(0, 'ArrowDown', 0, 5)).toBeNull()
    expect(isNavigationKey('a')).toBe(false)
  })
})

describe('tableKeyIntent', () => {
  it('maps Space, Escape and Enter', () => {
    expect(tableKeyIntent(' ', false)).toEqual({ kind: 'space' })
    expect(tableKeyIntent('Escape', false)).toEqual({ kind: 'escape' })
    expect(tableKeyIntent('Enter', false)).toEqual({ kind: 'open' })
  })

  it('maps F2 to rename, Delete to trash and Shift+Delete to permanent delete', () => {
    expect(tableKeyIntent('F2', false)).toEqual({ kind: 'manage', action: 'rename' })
    expect(tableKeyIntent('Delete', false)).toEqual({ kind: 'manage', action: 'trash' })
    expect(tableKeyIntent('Delete', true)).toEqual({ kind: 'manage', action: 'delete' })
  })

  it('moves with a navigation key and extends with Shift', () => {
    expect(tableKeyIntent('ArrowDown', false)).toEqual({ kind: 'move', key: 'ArrowDown' })
    expect(tableKeyIntent('ArrowDown', true)).toEqual({ kind: 'extend', key: 'ArrowDown' })
    expect(tableKeyIntent('End', true)).toEqual({ kind: 'extend', key: 'End' })
  })

  it('ignores every other key', () => {
    expect(tableKeyIntent('a', false)).toBeNull()
    expect(tableKeyIntent('Tab', true)).toBeNull()
  })
})

describe('nextActiveIndex in a grid of four columns', () => {
  const move = (from: number, key: string, count = 10, page = 2) =>
    nextActiveIndex(from, key, count, page, 4)

  it('moves sideways by one and stops at both ends', () => {
    expect(move(1, 'ArrowRight')).toBe(2)
    expect(move(3, 'ArrowRight')).toBe(4)
    expect(move(1, 'ArrowLeft')).toBe(0)
    expect(move(0, 'ArrowLeft')).toBe(0)
    expect(move(9, 'ArrowRight')).toBe(9)
  })

  it('moves a whole line up and down', () => {
    expect(move(1, 'ArrowDown')).toBe(5)
    expect(move(5, 'ArrowUp')).toBe(1)
    expect(move(1, 'ArrowUp')).toBe(1)
  })

  it('lands on the last card when the last line is shorter', () => {
    expect(move(7, 'ArrowDown')).toBe(9)
    expect(move(9, 'ArrowDown')).toBe(9)
    expect(move(8, 'ArrowDown')).toBe(8)
  })

  it('pages by whole lines', () => {
    expect(move(1, 'PageDown', 40)).toBe(9)
    expect(move(9, 'PageUp', 40)).toBe(1)
  })
})

describe('nextActiveIndex in the table', () => {
  it('leaves left and right to the sideways scrolling', () => {
    expect(nextActiveIndex(3, 'ArrowLeft', 10, 5)).toBeNull()
    expect(nextActiveIndex(3, 'ArrowRight', 10, 5)).toBeNull()
  })
})
