import { notifications } from '@mantine/notifications'
import { MutationCache, QueryClient } from '@tanstack/react-query'
import { describeError } from '../i18n/error-messages'
import { describeForLog } from '../ipc/ipc-error'
import { logError } from '../log/frontend-log'

// Every failed mutation (a user action) becomes a translated toast, so no action fails silently.
// A mutation with its own error UI opts out with `meta: { suppressErrorToast: true }`. Queries
// render their own error state instead.
export function createQueryClient(): QueryClient {
  return new QueryClient({
    mutationCache: new MutationCache({
      onError: (error, _variables, _onMutateResult, mutation) => {
        logError(`mutation failed: ${describeForLog(error)}`)
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
}

export const queryClient = createQueryClient()
