import { ipc } from '../../lib/ipc'
import { isIpcError } from '../../lib/ipc/ipc-error'
import type { ApplyDirection, ApplyRequest } from '../../lib/ipc/types'
import { showErrorToast } from '../../lib/query/error-toast'
import { routeFsError } from '../../shared/fs-error-route'
import type { ApplyHandlers, ApplyMutate } from './types'

// Switches without showing the change list first: the diff is fetched and applied with its
// fingerprint at once. A disk that changed in between calls `onStale` (the caller shows the
// list instead); a running game asks first and then sends the same request again.
export async function applyDirect(
  mutate: ApplyMutate,
  profileIds: string[],
  direction: ApplyDirection,
  handlers: ApplyHandlers,
): Promise<void> {
  let fingerprint: string
  try {
    fingerprint = (await ipc.profilesDiff(profileIds, direction)).fingerprint
  } catch (error) {
    showErrorToast(error)
    return
  }
  await send(mutate, { profileIds, fingerprint, applyWhileRunning: false, direction }, handlers)
}

async function send(
  mutate: ApplyMutate,
  request: ApplyRequest,
  handlers: ApplyHandlers,
): Promise<void> {
  try {
    handlers.onDone(await mutate(request))
  } catch (error) {
    if (isIpcError(error) && error.appError.kind === 'stale') {
      handlers.onStale()
      return
    }
    routeFsError(error, (applyWhileRunning) => {
      void send(mutate, { ...request, applyWhileRunning }, handlers)
    })
  }
}
