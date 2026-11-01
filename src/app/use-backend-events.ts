import { useQueryClient } from '@tanstack/react-query'
import { useEffect } from 'react'
import { onGameStatusChanged, onModsChanged } from '../lib/ipc/events'
import { describeForLog } from '../lib/ipc/ipc-error'
import type { GameStatus, ModsSnapshot } from '../lib/ipc/types'
import { logError } from '../lib/log/frontend-log'
import { applyGameStatusEvent } from '../shared/game/game-status-event'
import { queryKeys } from '../shared/query-keys'

// StrictMode mounts twice: each mount keeps its own pending subscriptions and releases them on
// cleanup, even when the listener resolves only after the cleanup ran.
export function useBackendEvents(): void {
  const queryClient = useQueryClient()

  useEffect(() => {
    let active = true
    const subscriptions = [
      onModsChanged(({ revision }) => {
        const cached = queryClient.getQueryData<ModsSnapshot>(queryKeys.mods)
        if (cached?.revision !== revision) {
          void queryClient.invalidateQueries({ queryKey: queryKeys.mods })
        }
      }),
      onGameStatusChanged((event) => {
        queryClient.setQueryData<GameStatus>(queryKeys.gameStatus, (status) =>
          applyGameStatusEvent(status, event),
        )
      }),
    ]
    // The backend does not buffer events, and the first fetches started before the listeners were
    // in place, so whatever changed in between is fetched once more.
    Promise.all(subscriptions).then(
      () => {
        if (!active) return
        void queryClient.invalidateQueries({ queryKey: queryKeys.gameStatus })
        void queryClient.invalidateQueries({ queryKey: queryKeys.mods })
      },
      (error: unknown) => logError(`event subscription failed: ${describeForLog(error)}`),
    )
    return () => {
      active = false
      for (const subscription of subscriptions) {
        // unlisten is async in practice, and the event plugin can be gone already (window
        // closing, test teardown), so a failure to release is ignored.
        subscription.then((unlisten) => Promise.resolve(unlisten())).catch(() => undefined)
      }
    }
  }, [queryClient])
}
