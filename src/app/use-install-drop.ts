import { notifications } from '@mantine/notifications'
import { useEffect, useState } from 'react'
import { i18n } from '../lib/i18n'
import { onInstallDrop } from '../lib/ipc/events'
import { describeForLog } from '../lib/ipc/ipc-error'
import type { InstallDrop, RejectedSource } from '../lib/ipc/types'
import { logError } from '../lib/log/frontend-log'
import { queuedFromPicked } from '../shared/install-actions'
import { useInstallQueue } from '../shared/install-queue'

const TOAST_MS = 10_000
const IDLE = { active: false, count: 0 }

function showRejected(rejected: readonly RejectedSource[]): void {
  const t = i18n.getFixedT(null, 'install')
  const lines = rejected.map(
    (entry) => `${entry.name}: ${t(($) => $.rejected.reasons[entry.reason])}`,
  )
  notifications.show({
    color: 'yellow',
    title: t(($) => $.rejected.title),
    message: lines.join('\n'),
    autoClose: TOAST_MS,
    styles: { description: { whiteSpace: 'pre-line', overflowWrap: 'anywhere' } },
  })
}

function handleDropped(event: InstallDrop): void {
  useInstallQueue.getState().push(event.sources.map(queuedFromPicked))
  if (event.rejected.length > 0) showRejected(event.rejected)
}

// The Rust side owns drag and drop (the webview never sees the paths): it reports the drag
// entering and leaving, and the classified sources on the drop.
export function useInstallDrop(): { active: boolean; count: number } {
  const [state, setState] = useState(IDLE)

  useEffect(() => {
    const subscription = onInstallDrop((event) => {
      if (event.phase === 'enter') {
        setState({ active: true, count: event.count })
        return
      }
      setState(IDLE)
      if (event.phase === 'dropped') handleDropped(event)
    })
    subscription.catch((error: unknown) =>
      logError(`install drop subscription failed: ${describeForLog(error)}`),
    )
    return () => {
      // The listener resolves asynchronously and the event plugin can be gone already.
      subscription.then((unlisten) => Promise.resolve(unlisten())).catch(() => undefined)
    }
  }, [])

  return state
}
