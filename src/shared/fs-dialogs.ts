import { modals } from '@mantine/modals'
import type { TFunction } from 'i18next'

export type DialogTranslate = TFunction<'errors'>

// Resolves true on confirm and false on cancel or any other way to close the dialog.
export function confirmApplyWhileRunning(t: DialogTranslate): Promise<boolean> {
  return new Promise((resolve) => {
    modals.openConfirmModal({
      title: t(($) => $.dialogs.running.title),
      children: t(($) => $.dialogs.running.body),
      labels: {
        confirm: t(($) => $.dialogs.running.confirm),
        cancel: t(($) => $.dialogs.running.cancel),
      },
      onConfirm: () => resolve(true),
      onCancel: () => resolve(false),
      onClose: () => resolve(false),
    })
  })
}

// With a retry callback the dialog offers "Try again"; without one it is a plain notice.
export function openBusyDialog(t: DialogTranslate, onRetry?: () => void): void {
  modals.openConfirmModal({
    title: t(($) => $.dialogs.busy.title),
    children: t(($) => $.dialogs.busy.body),
    labels: {
      confirm: onRetry ? t(($) => $.dialogs.busy.retry) : t(($) => $.dialogs.ok),
      cancel: t(($) => $.dialogs.busy.cancel),
    },
    cancelProps: onRetry ? undefined : { display: 'none' },
    onConfirm: onRetry,
  })
}

export function openStaleDialog(t: DialogTranslate): void {
  modals.openConfirmModal({
    title: t(($) => $.dialogs.stale.title),
    children: t(($) => $.dialogs.stale.body),
    labels: { confirm: t(($) => $.dialogs.ok), cancel: t(($) => $.dialogs.ok) },
    cancelProps: { display: 'none' },
  })
}
