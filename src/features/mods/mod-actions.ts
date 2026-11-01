import type { ModRow } from '../../lib/ipc/types'
import { bulkEnable, canToggle, rowsToChange } from './toggle-model'
import type { ModAction } from './types'

export function selectedRows(rows: readonly ModRow[], selection: readonly string[]): ModRow[] {
  const wanted = new Set(selection)
  return rows.filter((row) => wanted.has(row.modRef))
}

// One list for the ActionBar buttons and the keyboard: the counts are the rows each action would
// really change, and an action with nothing to change is disabled.
export function modActions(rows: readonly ModRow[], selection: readonly string[]): ModAction[] {
  const selected = selectedRows(rows, selection)
  const actions: ModAction[] = []
  for (const id of ['enable', 'disable'] as const) {
    const count = rowsToChange(selected, id === 'enable').length
    actions.push({ id, count, disabled: count === 0 })
  }
  return actions
}

// Space: a bulk toggle when the active row is one of two or more selected rows, else the active
// row alone. Blocked rows do nothing (null).
export function spaceAction(
  rows: readonly ModRow[],
  selection: readonly string[],
  activeRef: string | null,
): { enable: boolean; rows: ModRow[] } | null {
  if (activeRef === null) return null
  const active = rows.find((row) => row.modRef === activeRef)
  if (!active) return null
  if (selection.length >= 2 && selection.includes(activeRef)) {
    const toggleable = selectedRows(rows, selection).filter(canToggle)
    const enable = bulkEnable(toggleable)
    const changing = rowsToChange(toggleable, enable)
    return changing.length === 0 ? null : { enable, rows: changing }
  }
  if (!canToggle(active)) return null
  return { enable: rowsToChange([active], true).length === 1, rows: [active] }
}
