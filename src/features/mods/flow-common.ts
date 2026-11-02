import { isIpcError } from '../../lib/ipc/ipc-error'

// Thrown inside a flow when the user declines the "game is running" dialog.
export class Declined extends Error {}

export function kindOf(error: unknown): string | null {
  return isIpcError(error) ? error.appError.kind : null
}

// The first call goes without `applyWhileRunning`; a running game asks, then the same call is
// repeated with it. Declining ends the flow without a further call.
export async function withRunningRetry<T>(
  askRunning: () => Promise<boolean>,
  call: (applyWhileRunning: boolean) => Promise<T>,
): Promise<T> {
  try {
    return await call(false)
  } catch (error) {
    if (kindOf(error) !== 'gameRunning') throw error
    if (!(await askRunning())) throw new Declined()
    return call(true)
  }
}

export async function runGuarded(
  fail: (error: unknown) => void,
  flow: () => Promise<void>,
): Promise<void> {
  try {
    await flow()
  } catch (error) {
    if (error instanceof Declined) return
    fail(error)
  }
}
