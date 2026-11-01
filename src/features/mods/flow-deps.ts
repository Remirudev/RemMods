import type { QueryClient } from '@tanstack/react-query'
import { i18n } from '../../lib/i18n'
import { ipc } from '../../lib/ipc'
import type { GameStatus } from '../../lib/ipc/types'
import { confirmApplyWhileRunning } from '../../shared/fs-dialogs'
import { routeFsError } from '../../shared/fs-error-route'
import { storeModsSnapshot } from '../../shared/mods-cache'
import { queryKeys } from '../../shared/query-keys'
import { askFlowDialog } from './flow-dialogs'
import { showFlowNotice } from './flow-notices'
import type { FlowDeps } from './types'

export function originalAppRunning(client: QueryClient): boolean {
  return client.getQueryData<GameStatus>(queryKeys.gameStatus)?.originalApp ?? false
}

// The real dependencies of the trash and delete flows (the flows themselves stay pure).
export function buildFlowDeps(client: QueryClient): FlowDeps {
  return {
    api: {
      trash: (request) => ipc.modsTrash(request),
      prepare: (target) => ipc.modsDeletePrepare(target),
      remove: (request) => ipc.modsDeletePermanently(request),
    },
    ask: (dialog) => askFlowDialog(dialog, originalAppRunning(client)),
    askRunning: () => confirmApplyWhileRunning(i18n.getFixedT(null, 'errors')),
    store: (snapshot) => storeModsSnapshot(client, snapshot),
    notice: showFlowNotice,
    fail: (error) => routeFsError(error),
  }
}
