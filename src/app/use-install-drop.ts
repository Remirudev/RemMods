import { notifications } from '@mantine/notifications'
import { useEffect, useState } from 'react'
import { i18n } from '../lib/i18n'
import { onInstallDrop } from '../lib/ipc/events'
import { describeForLog } from '../lib/ipc/ipc-error'
import type { InstallDrop, PickedSource, RejectedSource } from '../lib/ipc/types'
import { logError } from '../lib/log/frontend-log'
import { showErrorToast } from '../lib/query/error-toast'
import { queuedFromPicked } from '../shared/install-actions'
import { useInstallQueue } from '../shared/install-queue'
import { usePictureDropStore } from '../shared/picture-drop-store'
import { isUpdating } from '../shared/update-run-store'

const TOAST_MS = 10_000
const IDLE = { active: false, count: 0, images: 0 }

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

async function dropPictures(images: readonly PickedSource[]): Promise<void> {
  const { target } = usePictureDropStore.getState()
  if (!target) {
    const t = i18n.getFixedT(null, 'install')
    notifications.show({
      color: 'yellow',
      title: t(($) => $.picture.noModTitle),
      message: t(($) => $.picture.noMod),
      autoClose: TOAST_MS,
    })
    return
  }
  try {
    for (const image of target.many ? images : images.slice(0, 1)) await target.apply(image.id)
  } catch (error) {
    showErrorToast(error)
  }
}

// Images are never installed: they go to whatever open view takes pictures.
function handleDropped(event: InstallDrop): void {
  const images = event.sources.filter((source) => source.kind === 'image')
  const installable = event.sources.filter((source) => source.kind !== 'image')
  useInstallQueue.getState().push(installable.map(queuedFromPicked))
  if (images.length > 0) void dropPictures(images)
  if (event.rejected.length > 0) showRejected(event.rejected)
}

// The Rust side owns drag and drop (the webview never sees the paths): it reports the drag
// entering and leaving, and the classified sources on the drop. While a self-update runs every
// drop is ignored (the sources stay unused in Rust's bounded registry).
export function useInstallDrop(): { active: boolean; count: number; images: number } {
  const [state, setState] = useState(IDLE)

  useEffect(() => {
    const subscription = onInstallDrop((event) => {
      if (isUpdating()) {
        setState(IDLE)
        return
      }
      if (event.phase === 'enter') {
        setState({ active: true, count: event.count, images: event.images })
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
