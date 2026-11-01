import { describe, expect, it } from 'vitest'
import { isNavigationKey, nextActiveIndex } from './table-keyboard'

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
