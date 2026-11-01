import { notifications } from '@mantine/notifications'
import { MutationCache, QueryCache, QueryClient } from '@tanstack/react-query'
import { storeModsSnapshot } from '../../shared/mods-cache'
import { queryKeys } from '../../shared/query-keys'
import { describeError } from '../i18n/error-messages'
import { ipc } from '../ipc'
import { describeForLog, isIpcError } from '../ipc/ipc-error'
import { logError } from '../log/frontend-log'

function isStale(error: unknown): boolean {
  return isIpcError(error) && error.appError.kind === 'stale'
}

// `stale` means the disk or the Steam candidates changed under a cached entry. The game status is
// fetched again; mods_list answers from the backend's cached scan, so the list is rescanned with
// mods_refresh instead, which is what the `stale` message promises.
function recoverFromStale(client: QueryClient): void {
  void client.invalidateQueries({ queryKey: queryKeys.gameStatus })
  if (client.getQueryData(queryKeys.mods) === undefined) return
  ipc.modsRefresh().then(
    (snapshot) => storeModsSnapshot(client, snapshot),
    (error: unknown) => logError(`mods_refresh after stale failed: ${describeForLog(error)}`),
  )
}

// Every failed mutation (a user action) becomes a translated toast, so no action fails silently.
// A mutation with its own error UI opts out with `meta: { suppressErrorToast: true }`. Queries
// render their own error state instead.
export function createQueryClient(): QueryClient {
  const client: QueryClient = new QueryClient({
    queryCache: new QueryCache({
      onError: (error) => {
        if (isStale(error)) recoverFromStale(client)
      },
    }),
    mutationCache: new MutationCache({
      onError: (error, _variables, _onMutateResult, mutation) => {
        logError(`mutation failed: ${describeForLog(error)}`)
        if (isStale(error)) recoverFromStale(client)
        if (mutation.meta?.suppressErrorToast) return
        const { title, message } = describeError(error)
        notifications.show({ color: 'red', title, message, autoClose: 6000 })
      },
    }),
    defaultOptions: {
      queries: {
        networkMode: 'always',
        retry: 0,
        staleTime: Number.POSITIVE_INFINITY,
        refetchOnWindowFocus: false,
      },
      mutations: {
        networkMode: 'always',
        retry: 0,
      },
    },
  })
  return client
}

export const queryClient = createQueryClient()
