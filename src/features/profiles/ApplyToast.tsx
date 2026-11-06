import { Button, Group, Text } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { i18n } from '../../lib/i18n'
import type { ApplyOutcome } from '../../lib/ipc/types'
import { openStaleDialog } from '../../shared/fs-dialogs'
import { applyDirect } from './group-apply'
import type { ApplyToastProps } from './types'
import { useApplyProfile } from './use-profile-mutations'

const TOAST_MS = 10_000

// The result of a switch. Undo applies the automatic snapshot taken just before, which puts
// every mod back the way it was.
export function ApplyToast({ notificationId, outcome }: ApplyToastProps) {
  const { t } = useTranslation('profiles')
  const { mutateAsync } = useApplyProfile()
  const [undoing, setUndoing] = useState(false)
  const { snapshotId, skins } = outcome
  const message = [
    outcome.changed > 0
      ? t(($) => $.apply.done, { count: outcome.changed })
      : t(($) => $.apply.nothing),
    outcome.skipped.length > 0
      ? t(($) => $.apply.skipped, { count: outcome.skipped.length })
      : null,
    skins?.kind === 'several' ? t(($) => $.apply.skins.several) : null,
    skins && skins.kind !== 'several'
      ? t(($) => $.apply.skins[skins.kind], { name: skins.name })
      : null,
  ]
    .filter(Boolean)
    .join(' ')

  const undo = () => {
    if (snapshotId === null) return
    setUndoing(true)
    void applyDirect(mutateAsync, [snapshotId], 'on', {
      onDone: (next) => {
        notifications.hide(notificationId)
        showApplyToast(next)
      },
      onStale: () => openStaleDialog(i18n.getFixedT(null, 'errors')),
    }).finally(() => setUndoing(false))
  }

  return (
    <Group justify="space-between" wrap="nowrap" gap="sm">
      <Text size="sm">{message}</Text>
      {snapshotId !== null && (
        <Button size="compact-sm" variant="light" loading={undoing} onClick={undo}>
          {t(($) => $.apply.undo)}
        </Button>
      )}
    </Group>
  )
}

export function showApplyToast(outcome: ApplyOutcome): void {
  const id = `apply-${crypto.randomUUID()}`
  notifications.show({
    id,
    message: <ApplyToast notificationId={id} outcome={outcome} />,
    autoClose: TOAST_MS,
  })
}
