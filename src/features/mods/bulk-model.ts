import type {
  BulkDeletePrepared,
  BulkRow,
  ModTarget,
  PreparedRow,
  TrashKind,
} from '../../lib/ipc/types'
import { canManage } from './manage-model'
import type { ManageSubject } from './types'

export function sameTarget(a: ModTarget, b: ModTarget): boolean {
  return a.modRef === b.modRef && a.key === b.key
}

export function subjectOf(
  row: BulkRow,
  subjects: readonly ManageSubject[] | undefined,
): ManageSubject | undefined {
  return subjects?.find((subject) => sameTarget(subject.target, row.target))
}

// The rows of a bulk trash the system Trash refused that the matrix also lets RemMods delete:
// the only rows the results dialog offers for a permanent delete.
export function deletableUnavailable(
  rows: readonly BulkRow[],
  subjects: readonly ManageSubject[] | undefined,
): BulkRow[] {
  return rows.filter((row) => {
    if (row.result.kind !== 'unavailable') return false
    const subject = subjectOf(row, subjects)
    return subject !== undefined && canManage(subject, 'delete')
  })
}

export function unavailableSize(row: BulkRow, subject: ManageSubject | undefined): number | null {
  if (row.result.kind !== 'unavailable') return null
  return row.result.size ?? subject?.sizeBytes ?? null
}

// Unknown sizes are left out of a total; a total of nothing known is unknown.
export function totalSize(sizes: readonly (number | null)[]): number | null {
  const known = sizes.filter((size): size is number => size !== null)
  return known.length === 0 ? null : known.reduce((sum, size) => sum + size, 0)
}

export function eligibleRows(rows: readonly PreparedRow[]): PreparedRow[] {
  return rows.filter((row) => row.refusal === null)
}

export function eligibleTargets(prepared: BulkDeletePrepared): ModTarget[] {
  return eligibleRows(prepared.rows).map((row) => row.target)
}

// Prepare refusals reach the results list as failed rows, so nothing is silently dropped.
export function refusalsAsRows(prepared: BulkDeletePrepared): BulkRow[] {
  const rows: BulkRow[] = []
  for (const row of prepared.rows) {
    if (row.refusal === null) continue
    rows.push({
      target: row.target,
      name: row.name,
      result: { kind: 'failed', error: row.refusal },
    })
  }
  return rows
}

export function sameTargets(a: readonly ModTarget[], b: readonly ModTarget[]): boolean {
  return (
    a.length === b.length && a.every((target, index) => sameTarget(target, b[index] as ModTarget))
  )
}

export function trashKindOf(rows: readonly BulkRow[]): TrashKind {
  for (const row of rows) {
    if (row.result.kind === 'trashed') return row.result.trash
  }
  return 'home'
}
