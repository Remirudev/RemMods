import type { LeftoverEntry, ModRow, RootInfo } from '../../lib/ipc/types'

export function rowsWithIssues(rows: readonly ModRow[]): ModRow[] {
  return rows.filter((row) => row.issues.length > 0)
}

export function rootsWithProblems(roots: readonly RootInfo[]): RootInfo[] {
  return roots.filter((root) => root.caseVariant || root.unreadable !== null)
}

// What the Issues tab lists: mods with issues, roots with problems and one entry per leftover.
export function countIssues(
  rows: readonly ModRow[],
  roots: readonly RootInfo[],
  leftovers: readonly LeftoverEntry[],
): number {
  return rowsWithIssues(rows).length + rootsWithProblems(roots).length + leftovers.length
}
