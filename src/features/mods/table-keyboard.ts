import type { NavigationKey } from './types'

const NAVIGATION_KEYS: ReadonlySet<string> = new Set<NavigationKey>([
  'ArrowUp',
  'ArrowDown',
  'Home',
  'End',
  'PageUp',
  'PageDown',
])

export function isNavigationKey(key: string): key is NavigationKey {
  return NAVIGATION_KEYS.has(key)
}

// The row the grid's active row moves to for a navigation key, or null when the key is not one
// (or there are no rows). `current` is -1 while no row is active; a stale index past the end
// (after filtering) counts as the last row.
export function nextActiveIndex(
  current: number,
  key: string,
  count: number,
  pageSize: number,
): number | null {
  if (count === 0 || !isNavigationKey(key)) return null
  const last = count - 1
  const page = Math.max(1, pageSize)
  if (current < 0) return key === 'End' ? last : 0
  const from = Math.min(current, last)
  switch (key) {
    case 'ArrowDown':
      return Math.min(from + 1, last)
    case 'ArrowUp':
      return Math.max(from - 1, 0)
    case 'Home':
      return 0
    case 'End':
      return last
    case 'PageDown':
      return Math.min(from + page, last)
    case 'PageUp':
      return Math.max(from - page, 0)
  }
}
