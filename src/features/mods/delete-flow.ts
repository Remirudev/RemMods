import { isIpcError } from '../../lib/ipc/ipc-error'
import type { DeletePrepared } from '../../lib/ipc/types'
import type { FlowDeps, ManageSubject } from './types'

// Thrown inside a flow when the user declines the "game is running" dialog.
class Declined extends Error {}

function kindOf(error: unknown): string | null {
  return isIpcError(error) ? error.appError.kind : null
}

// The first call goes without `applyWhileRunning`; a running game asks, then the same call is
// repeated with it. Declining ends the flow without a further call.
async function withRunningRetry<T>(
  deps: FlowDeps,
  call: (applyWhileRunning: boolean) => Promise<T>,
): Promise<T> {
  try {
    return await call(false)
  } catch (error) {
    if (kindOf(error) !== 'gameRunning') throw error
    if (!(await deps.askRunning())) throw new Declined()
    return call(true)
  }
}

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

async function runGuarded(deps: FlowDeps, flow: () => Promise<void>): Promise<void> {
  try {
    await flow()
  } catch (error) {
    if (error instanceof Declined) return
    deps.fail(error)
  }
}

// Trash first; where the system Trash cannot be used the user gets a second, red confirm for a
// permanent delete. Nothing is deleted without that second confirm.
export function trashWithFallback(subject: ManageSubject, deps: FlowDeps): Promise<void> {
  return runGuarded(deps, async () => {
    if (!(await deps.ask({ kind: 'trash', subject }))) return
    try {
      const outcome = await withRunningRetry(deps, (applyWhileRunning) =>
        deps.api.trash({ target: subject.target, applyWhileRunning }),
      )
      deps.store(outcome.snapshot)
      deps.notice({ kind: 'trashDone', name: subject.name })
    } catch (error) {
      if (error instanceof Declined) return
      await trashFails(subject, deps, error)
    }
  })
}

// Two confirms, with the token minted between them.
export function deletePermanently(subject: ManageSubject, deps: FlowDeps): Promise<void> {
  return runGuarded(deps, async () => {
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
