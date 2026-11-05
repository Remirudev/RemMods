import { i18n } from '../lib/i18n'
import { isIpcError } from '../lib/ipc/ipc-error'
import { showErrorToast } from '../lib/query/error-toast'
import { confirmApplyWhileRunning, openBusyDialog, openStaleDialog } from './fs-dialogs'

// One scope for everything that renames files: requests run one after another.
export const FS_SCOPE = { id: 'fs' }

// The failure paths of every file-changing mutation. The mutation opts out of the global toast,
// so each outcome is handled here: the running game asks first, then the same request is sent
// again with `applyWhileRunning`. Without `retry` both fall back to a notice.
export function routeFsError(error: unknown, retry?: (applyWhileRunning: boolean) => void): void {
  const t = i18n.getFixedT(null, 'errors')
  const kind = isIpcError(error) ? error.appError.kind : null
  if (kind === 'gameRunning' && retry) {
    void confirmApplyWhileRunning(t).then((confirmed) => {
      if (confirmed) retry(true)
    })
  } else if (kind === 'busy') {
    openBusyDialog(t, retry ? () => retry(false) : undefined)
  } else if (kind === 'stale') {
    openStaleDialog(t)
  } else {
    showErrorToast(error)
  }
}
