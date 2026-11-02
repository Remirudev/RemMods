import { MutationObserver, type QueryClient } from '@tanstack/react-query'
import { i18n } from '../../lib/i18n'
import { ipc } from '../../lib/ipc'
import { confirmApplyWhileRunning } from '../../shared/fs-dialogs'
import { FS_SCOPE, routeFsError } from '../../shared/fs-error-route'
import { storeOrRefetchMods } from '../../shared/mods-cache'
import { askBulkDialog, showBulkResults } from './bulk-dialogs'
import { showBulkNotice } from './bulk-notices'
import type { BulkDeps } from './types'

// The bulk calls change files, so they queue in the serial `fs` scope behind any other change.
// The flow reports their errors itself, so the global toast stays off.
function inFsScope<V, R>(client: QueryClient, call: (variables: V) => Promise<R>) {
  return (variables: V): Promise<R> =>
    new MutationObserver<R, unknown, V>(client, {
      scope: FS_SCOPE,
      meta: { suppressErrorToast: true },
      mutationFn: call,
    }).mutate(variables)
}

export function buildBulkDeps(client: QueryClient): BulkDeps {
  return {
    api: {
      trashMany: inFsScope(client, (request) => ipc.modsTrashMany(request)),
      prepareMany: (targets) => ipc.modsDeletePrepareMany(targets),
      deleteMany: inFsScope(client, (request) => ipc.modsDeleteMany(request)),
    },
    ask: (dialog) => askBulkDialog(dialog),
    askRunning: () => confirmApplyWhileRunning(i18n.getFixedT(null, 'errors')),
    store: (snapshot) => storeOrRefetchMods(client, snapshot),
    notice: showBulkNotice,
    showResults: (results) => showBulkResults(results),
    fail: (error) => routeFsError(error),
  }
}
