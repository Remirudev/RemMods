import type { BulkDeletePrepared, BulkOutcome } from '../../lib/ipc/types'
import {
  deletableUnavailable,
  eligibleTargets,
  refusalsAsRows,
  sameTargets,
  trashKindOf,
} from './bulk-model'
import { kindOf, runGuarded, withRunningRetry } from './flow-common'
import type { BulkDeps, BulkSelection } from './types'

function cleanDelete(outcome: BulkOutcome): boolean {
  return outcome.rows.every((row) => row.result.kind === 'deleted' && row.result.leftover === null)
}

// One token covers exactly the ordered eligible rows. Every call spends it, a refused one too: a
// running game gets a fresh prepare and exactly one more call, with every confirm already given.
async function removeMany(
  prepared: BulkDeletePrepared,
  deps: BulkDeps,
  applyWhileRunning: boolean,
): Promise<void> {
  const token = prepared.token
  if (token === null) {
    deps.notice({ kind: 'bulkNothingEligible' })
    return
  }
  const targets = eligibleTargets(prepared)
  try {
    const outcome = await deps.api.deleteMany({ token, targets, applyWhileRunning })
    deps.store(outcome.snapshot)
    const refused = refusalsAsRows(prepared)
    if (refused.length === 0 && cleanDelete(outcome)) {
      deps.notice({ kind: 'bulkDeleteDone', count: outcome.rows.length })
      return
    }
    await deps.showResults({ action: 'delete', rows: [...outcome.rows, ...refused] })
  } catch (error) {
    const kind = kindOf(error)
    if (kind === 'deleteTokenInvalid') {
      deps.notice({ kind: 'deleteExpired' })
    } else if (kind === 'gameRunning' && !applyWhileRunning) {
      if (!(await deps.askRunning())) return
      const fresh = await deps.api.prepareMany(targets)
      await removeMany(fresh, deps, true)
    } else {
      throw error
    }
  }
}

// Bulk trash: one confirm, one call. Rows the Trash refused are listed with their reasons and
// sizes; the results dialog's "Delete permanently (K)" is the second confirmation, and nothing is
// deleted without it.
export function trashMany(selection: BulkSelection, deps: BulkDeps): Promise<void> {
  const { subjects, skipped } = selection
  return runGuarded(deps.fail, async () => {
    if (subjects.length === 0) return
    if (!(await deps.ask({ kind: 'bulkTrash', subjects, skipped }))) return
    const targets = subjects.map((subject) => subject.target)
    const outcome = await withRunningRetry(deps.askRunning, (applyWhileRunning) =>
      deps.api.trashMany({ targets, applyWhileRunning }),
    )
    deps.store(outcome.snapshot)
    if (outcome.rows.every((row) => row.result.kind === 'trashed')) {
      deps.notice({
        kind: 'bulkTrashDone',
        count: outcome.rows.length,
        trash: trashKindOf(outcome.rows),
      })
      return
    }
    const answer = await deps.showResults({ action: 'trash', rows: outcome.rows, subjects })
    const shown = deletableUnavailable(outcome.rows, subjects).map((row) => row.target)
    if (answer !== 'deleteUnavailable' || shown.length === 0) return
    const prepared = await deps.api.prepareMany(shown)
    if (prepared.token === null) {
      deps.notice({ kind: 'bulkNothingEligible' })
      return
    }
    const changed =
      prepared.rows.some((row) => row.refusal !== null) ||
      !sameTargets(eligibleTargets(prepared), shown)
    if (changed && !(await deps.ask({ kind: 'bulkDeleteFinal', rows: prepared.rows }))) return
    await removeMany(prepared, deps, false)
  })
}

// Two confirms with the token minted between them; the second lists exactly what the token covers.
export function deleteMany(selection: BulkSelection, deps: BulkDeps): Promise<void> {
  const { subjects, skipped } = selection
  return runGuarded(deps.fail, async () => {
    if (subjects.length === 0) return
    if (!(await deps.ask({ kind: 'bulkDeleteFirst', subjects, skipped }))) return
    const prepared = await deps.api.prepareMany(subjects.map((subject) => subject.target))
    if (prepared.token === null) {
      await deps.showResults({ action: 'delete', rows: refusalsAsRows(prepared) })
      return
    }
    if (!(await deps.ask({ kind: 'bulkDeleteFinal', rows: prepared.rows }))) return
    await removeMany(prepared, deps, false)
  })
}
