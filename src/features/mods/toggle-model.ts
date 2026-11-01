import type { ModRow, ModsSnapshot, ToggleTarget } from '../../lib/ipc/types'
import { displayState } from './display-state'

// A prefixed LogicMods folder is off even while its paks are live: the game reads the name only
// for the folder, and the toggle renames both.
export function isOn(row: ModRow): boolean {
  if (row.kind === 'logic' && row.logicPrefixed) return false
  return row.state === 'enabled' || row.state === 'partial'
}

export function canToggle(row: ModRow): boolean {
  return row.toggleBlock === null
}

export function toggleTargets(rows: readonly ModRow[]): ToggleTarget[] {
  return rows.map((row) => ({ modRef: row.modRef, key: row.key }))
}

// What a toggle to `enable` would change: toggleable rows that are not in that state yet.
export function rowsToChange(rows: readonly ModRow[], enable: boolean): ModRow[] {
  return rows.filter((row) => canToggle(row) && isOn(row) !== enable)
}

// False when every toggleable row is on (the next step is to turn them off), true otherwise.
export function bulkEnable(rows: readonly ModRow[]): boolean {
  return !rows.filter(canToggle).every(isOn)
}

function toggledRow(row: ModRow, enable: boolean): ModRow {
  const { livePak, offPak, liveCompanion, offCompanion } = row.payload
  const payload = enable
    ? {
        livePak: livePak + offPak,
        offPak: 0,
        liveCompanion: liveCompanion + offCompanion,
        offCompanion: 0,
      }
    : {
        livePak: 0,
        offPak: livePak + offPak,
        liveCompanion: 0,
        offCompanion: liveCompanion + offCompanion,
      }
  return {
    ...row,
    state: enable ? 'enabled' : 'disabled',
    payload,
    logicPrefixed: row.kind === 'logic' ? !enable : row.logicPrefixed,
  }
}

// The table answers at once; the backend's snapshot replaces this (or the rollback restores the
// previous one). Only rows that can change are touched.
export function optimisticSnapshot(
  snapshot: ModsSnapshot,
  refs: readonly string[],
  enable: boolean,
): ModsSnapshot {
  const wanted = new Set(refs)
  const rows = snapshot.rows.map((row) =>
    wanted.has(row.modRef) && canToggle(row) ? toggledRow(row, enable) : row,
  )
  return { ...snapshot, rows }
}

// "Disable all" also turns off LogicMods folders that are disabled by name only but still loaded.
export function disableAllCount(rows: readonly ModRow[]): number {
  return rows.filter(
    (row) => canToggle(row) && (isOn(row) || displayState(row) === 'disabledLoaded'),
  ).length
}

// Rows that were on in `before` and are off in `after`, matched by key (a Logic toggle renames
// the folder, so the ref changes).
export function disabledBetween(before: ModsSnapshot, after: ModsSnapshot): number {
  const offAfter = new Set(after.rows.filter((row) => !isOn(row)).map((row) => row.key))
  return before.rows.filter((row) => isOn(row) && offAfter.has(row.key)).length
}

// The one-click repair of a Logic row whose folder name and pak files disagree.
export function issueFix(row: ModRow): boolean | null {
  if (!canToggle(row)) return null
  for (const issue of row.issues) {
    if (issue.kind === 'logicDisabledButLoaded') return false
    if (issue.kind === 'logicEnabledNotLoaded') return true
  }
  return null
}
