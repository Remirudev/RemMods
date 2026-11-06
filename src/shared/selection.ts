import type { ClickModifiers, SelectionState } from './types'

function rangeBetween(ids: readonly string[], from: string, to: string): string[] | null {
  const start = ids.indexOf(from)
  const end = ids.indexOf(to)
  if (start < 0 || end < 0) return null
  return ids.slice(Math.min(start, end), Math.max(start, end) + 1)
}

// `ids` are the visible rows in display order. Shift keeps the anchor; every other click moves it.
export function applyClick(
  selection: readonly string[],
  anchor: string | null,
  ids: readonly string[],
  id: string,
  { shift, ctrl }: ClickModifiers,
): SelectionState {
  if (shift && anchor !== null) {
    const range = rangeBetween(ids, anchor, id)
    if (range) {
      const merged = ctrl ? [...new Set([...selection, ...range])] : range
      return { selection: merged, anchor }
    }
  }
  if (ctrl) {
    const next = selection.includes(id)
      ? selection.filter((candidate) => candidate !== id)
      : [...selection, id]
    return { selection: next, anchor: id }
  }
  return { selection: [id], anchor: id }
}

// Shift+Arrow: the selection becomes the range from the anchor to the row the cursor moved to.
export function extendTo(
  selection: readonly string[],
  anchor: string | null,
  ids: readonly string[],
  id: string,
): SelectionState {
  const from = anchor ?? id
  const range = rangeBetween(ids, from, id)
  if (!range) return { selection: [...selection], anchor }
  return { selection: range, anchor: from }
}

// Returns the same array when nothing was dropped, so a store update can be skipped.
export function pruneSelection(selection: readonly string[], ids: readonly string[]): string[] {
  const present = new Set(ids)
  const kept = selection.filter((id) => present.has(id))
  return kept.length === selection.length ? (selection as string[]) : kept
}
