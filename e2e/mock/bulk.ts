import type {
  AppError,
  BulkDeletePrepared,
  BulkDeleteRequest,
  BulkOutcome,
  BulkRow,
  BulkTrashRequest,
  ModRow,
  ModTarget,
  PreparedRow,
} from '../../src/lib/ipc/types'
import { consumeToken, issueToken } from './mods'
import { changed, fail, findRow, removeRow, targetOf } from './state'
import type { MockState } from './types'

const MAX_TARGETS = 1000

// Core's BulkTargets refusal: an empty list, too many targets or the same row twice.
function checkTargets(targets: readonly ModTarget[]): void {
  const refs = new Set(targets.map((target) => target.modRef))
  if (targets.length === 0 || targets.length > MAX_TARGETS || refs.size !== targets.length) {
    fail({ kind: 'invalidInput', field: 'targets' })
  }
}

function currentRow(state: MockState, target: ModTarget): ModRow | null {
  const row = findRow(state, target.modRef)
  return row && row.key === target.key ? row : null
}

function deleteRefusal(row: ModRow | null): AppError | null {
  if (!row) return { kind: 'stale' }
  if (row.kind === 'loose') return { kind: 'manageBlocked', action: 'delete', reason: 'loose' }
  return null
}

function staleRow(target: ModTarget): BulkRow {
  return { target, name: target.key, result: { kind: 'failed', error: { kind: 'stale' } } }
}

// Per-row results in request order; one rescan (one new revision) for the whole call.
export function trashMany(state: MockState, request: BulkTrashRequest): BulkOutcome {
  checkTargets(request.targets)
  const rows = request.targets.map((target): BulkRow => {
    const row = currentRow(state, target)
    if (!row) return staleRow(target)
    const refusal = state.trashRefusals[row.key]
    if (refusal) {
      return {
        target,
        name: row.name,
        result: { kind: 'unavailable', reason: refusal.reason, size: row.sizeBytes },
      }
    }
    removeRow(state, row)
    state.trashed.push(row.key)
    return { target, name: row.name, result: { kind: 'trashed', items: 1, trash: state.trashKind } }
  })
  return { snapshot: changed(state), rows }
}

export function deletePrepareMany(state: MockState, targets: ModTarget[]): BulkDeletePrepared {
  checkTargets(targets)
  const rows = targets.map((target): PreparedRow => {
    const row = currentRow(state, target)
    return {
      target,
      name: row?.name ?? target.key,
      size: row?.sizeBytes ?? null,
      refusal: deleteRefusal(row),
    }
  })
  const eligible = rows.filter((row) => row.refusal === null)
  const token =
    eligible.length === 0
      ? null
      : issueToken(
          state,
          eligible.map((row) => row.target),
        )
  return { token, rows, expiresInSecs: 120 }
}

export function deleteMany(state: MockState, request: BulkDeleteRequest): BulkOutcome {
  checkTargets(request.targets)
  consumeToken(state, request.token, request.targets)
  const rows = request.targets.map((target): BulkRow => {
    const row = currentRow(state, target)
    if (!row) return staleRow(target)
    removeRow(state, row)
    state.deleted.push(row.key)
    return { target: targetOf(row), name: row.name, result: { kind: 'deleted', leftover: null } }
  })
  return { snapshot: changed(state), rows }
}
