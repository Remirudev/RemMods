import { notifications } from '@mantine/notifications'
import { createElement, useEffect } from 'react'
import { i18n } from '../lib/i18n'
import { ipc } from '../lib/ipc'
import { onFolderBusy } from '../lib/ipc/events'
import { describeForLog } from '../lib/ipc/ipc-error'
import type { FolderBusy } from '../lib/ipc/types'
import { logError } from '../lib/log/frontend-log'
import { FolderBusyMessage } from './FolderBusyMessage'

interface WaitState {
  info: FolderBusy
  cancelling: boolean
  // Where the focus was before the toast took it; it goes back there when the wait ends.
  returnTo: Element | null
}

const TOAST_SELECTOR = '[data-testid="folder-busy"]'

export function folderBusyNotificationId(waitId: number): string {
  return `folder-busy-${waitId}`
}

function inToast(element: Element | null): boolean {
  return element?.closest(TOAST_SELECTOR) != null
}

// A second wait that starts while the first toast has the focus inherits the first one's target.
function focusTarget(waits: ReadonlyMap<number, WaitState>): Element | null {
  const active = document.activeElement
  if (!inToast(active)) return active
  for (const state of waits.values()) if (state.returnTo !== null) return state.returnTo
  return null
}

// The toast's Cancel held the focus; once it is gone or disabled the focus would fall to the
// body and the keyboard grid would stop answering, so it returns to where it was.
function restoreFocus(state: WaitState | undefined): void {
  const active = document.activeElement
  if (active !== null && active !== document.body && !inToast(active)) return
  const target = state?.returnTo
  if (target instanceof HTMLElement && target.isConnected) target.focus({ preventScroll: true })
}

// WG-2: while a folder rename waits for another program, a toast shows the wait with a Cancel
// that calls `mods_busy_cancel` directly (never a mutation: the `fs` scope is busy with the very
// call that waits). Any end phase hides it; the waiting call then reports its own result.
export function useFolderBusy(): void {
  useEffect(() => {
    let active = true
    const waits = new Map<number, WaitState>()

    const render = (state: WaitState, first: boolean) => {
      const waitId = state.info.waitId
      const notification = {
        id: folderBusyNotificationId(waitId),
        title: i18n.getFixedT(null, 'mods')(($) => $.folderBusy.title),
        message: createElement(FolderBusyMessage, {
          info: state.info,
          cancelling: state.cancelling,
          focusOnMount: first,
          onCancel: () => cancel(waitId),
        }),
        color: 'yellow',
        autoClose: false as const,
        withCloseButton: false,
      }
      if (first) notifications.show(notification)
      else notifications.update(notification)
    }

    const cancel = (waitId: number) => {
      const state = waits.get(waitId)
      if (!state || state.cancelling) return
      state.cancelling = true
      render(state, false)
      ipc.modsBusyCancel(waitId).catch((error: unknown) => {
        logError(`mods_busy_cancel failed: ${describeForLog(error)}`)
        state.cancelling = false
        if (waits.get(waitId) === state) render(state, false)
      })
    }

    const handle = (info: FolderBusy) => {
      if (!active) return
      if (info.phase !== 'waiting') {
        const ended = waits.get(info.waitId)
        waits.delete(info.waitId)
        restoreFocus(ended)
        notifications.hide(folderBusyNotificationId(info.waitId))
        return
      }
      const known = waits.get(info.waitId)
      const state = {
        info,
        cancelling: known?.cancelling ?? false,
        returnTo: known ? known.returnTo : focusTarget(waits),
      }
      waits.set(info.waitId, state)
      render(state, known === undefined)
    }

    const subscription = onFolderBusy(handle)
    subscription.catch((error: unknown) =>
      logError(`folder-busy subscription failed: ${describeForLog(error)}`),
    )
    return () => {
      active = false
      subscription.then((unlisten) => Promise.resolve(unlisten())).catch(() => undefined)
    }
  }, [])
}
