import { isIpcError } from '../../lib/ipc/ipc-error'
import type { DeletePrepared } from '../../lib/ipc/types'
import { Declined, kindOf, runGuarded, withRunningRetry } from './flow-common'
import type { FlowDeps, ManageSubject } from './types'

// The token is spent by every call, including a refused one: a game that runs gets a fresh
// prepare and exactly one more call, with both confirms already given (SPEC-P3 §7.2).
async function removeWithToken(
  subject: ManageSubject,
  deps: FlowDeps,
  prepared: DeletePrepared,
  applyWhileRunning: boolean,
): Promise<void> {
  try {
    const outcome = await deps.api.remove({
      target: subject.target,
      token: prepared.token,
      applyWhileRunning,
    })
    deps.store(outcome.snapshot)
    deps.notice({ kind: 'deleteDone', name: subject.name })
    if (outcome.leftover !== null) deps.notice({ kind: 'deleteLeftover', path: outcome.leftover })
  } catch (error) {
    const kind = kindOf(error)
    if (kind === 'deleteTokenInvalid') {
      deps.notice({ kind: 'deleteExpired' })
    } else if (kind === 'gameRunning' && !applyWhileRunning) {
      if (!(await deps.askRunning())) return
      const fresh = await deps.api.prepare(subject.target)
      await removeWithToken(subject, deps, fresh, true)
    } else {
      throw error
    }
  }
}

async function trashFails(subject: ManageSubject, deps: FlowDeps, error: unknown): Promise<void> {
  if (!isIpcError(error) || error.appError.kind !== 'trashUnavailable') throw error
  const { reason, size } = error.appError
  const confirmed = await deps.ask({
    kind: 'trashUnavailable',
    subject,
    reason,
    size: size ?? subject.sizeBytes,
  })
  if (!confirmed) return
  const prepared = await deps.api.prepare(subject.target)
  await removeWithToken(subject, deps, prepared, false)
}

// Trash first; where the system Trash cannot be used the user gets a second, red confirm for a
// permanent delete. Nothing is deleted without that second confirm.
export function trashWithFallback(subject: ManageSubject, deps: FlowDeps): Promise<void> {
  return runGuarded(deps.fail, async () => {
    if (!(await deps.ask({ kind: 'trash', subject }))) return
    try {
      const outcome = await withRunningRetry(deps.askRunning, (applyWhileRunning) =>
        deps.api.trash({ target: subject.target, applyWhileRunning }),
      )
      deps.store(outcome.snapshot)
      deps.notice({ kind: 'trashDone', name: subject.name, trash: outcome.kind })
    } catch (error) {
      if (error instanceof Declined) return
      await trashFails(subject, deps, error)
    }
  })
}

// Two confirms, with the token minted between them.
export function deletePermanently(subject: ManageSubject, deps: FlowDeps): Promise<void> {
  return runGuarded(deps.fail, async () => {
    if (!(await deps.ask({ kind: 'deleteFirst', subject }))) return
    const prepared = await deps.api.prepare(subject.target)
    const confirmed = await deps.ask({
      kind: 'deleteFinal',
      subject,
      name: prepared.name,
      size: prepared.size ?? subject.sizeBytes,
    })
    if (!confirmed) return
    await removeWithToken(subject, deps, prepared, false)
  })
}
