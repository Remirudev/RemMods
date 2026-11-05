import type { NavigationKey, TableKeyIntent } from './types'

const NAVIGATION_KEYS: ReadonlySet<string> = new Set<NavigationKey>([
  'ArrowUp',
  'ArrowDown',
  'ArrowLeft',
  'ArrowRight',
  'Home',
  'End',
  'PageUp',
  'PageDown',
])

export function isNavigationKey(key: string): key is NavigationKey {
  return NAVIGATION_KEYS.has(key)
}

// One line down: straight below when there is an item, else the last item if it sits on a later
// line, else nowhere.
function oneLineDown(from: number, last: number, columns: number): number {
  if (from + columns <= last) return from + columns
  return Math.floor(last / columns) > Math.floor(from / columns) ? last : from
}

// The item the active item moves to for a navigation key, or null when the key is not one (or
// there are no items). `current` is -1 while none is active; a stale index past the end (after
// filtering) counts as the last item. `columns` is 1 in the table, where left and right stay free
// to scroll sideways; `pageSize` counts lines.
export function nextActiveIndex(
  current: number,
  key: string,
  count: number,
  pageSize: number,
  columns = 1,
): number | null {
  if (count === 0 || !isNavigationKey(key)) return null
  if (columns === 1 && (key === 'ArrowLeft' || key === 'ArrowRight')) return null
  const last = count - 1
  const page = Math.max(1, pageSize) * columns
  if (current < 0) return key === 'End' ? last : 0
  const from = Math.min(current, last)
  switch (key) {
    case 'ArrowDown':
      return oneLineDown(from, last, columns)
    case 'ArrowUp':
      return from >= columns ? from - columns : from
    case 'ArrowRight':
      return Math.min(from + 1, last)
    case 'ArrowLeft':
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

// Shift+Arrow/Home/End/Page extends the selection; the same keys alone only move the active row.
export function tableKeyIntent(key: string, shift: boolean): TableKeyIntent | null {
  if (key === ' ') return { kind: 'space' }
  if (key === 'Escape') return { kind: 'escape' }
  if (key === 'Enter') return { kind: 'open' }
  if (key === 'F2') return { kind: 'manage', action: 'rename' }
  if (key === 'Delete') return { kind: 'manage', action: shift ? 'delete' : 'trash' }
  if (!isNavigationKey(key)) return null
  return shift ? { kind: 'extend', key } : { kind: 'move', key }
}
