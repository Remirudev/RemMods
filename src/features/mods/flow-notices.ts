import { notifications } from '@mantine/notifications'
import { i18n } from '../../lib/i18n'
import { placeOfKind } from '../../shared/trash-place'
import type { FlowNotice } from './types'

export function showFlowNotice(notice: FlowNotice): void {
  const t = i18n.getFixedT(null, 'mods')
  switch (notice.kind) {
    case 'trashDone':
      notifications.show({
        message:
          placeOfKind(notice.trash) === 'recycleBin'
            ? t(($) => $.trash.doneRecycle, { name: notice.name })
            : t(($) => $.trash.done, { name: notice.name }),
        autoClose: 6000,
      })
      return
    case 'deleteDone':
      notifications.show({
        message: t(($) => $.delete.done, { name: notice.name }),
        autoClose: 6000,
      })
      return
    case 'deleteExpired':
      notifications.show({ color: 'yellow', message: t(($) => $.delete.expired), autoClose: 8000 })
      return
    case 'deleteLeftover':
      notifications.show({
        color: 'yellow',
        message: t(($) => $.delete.leftover, { path: notice.path }),
        autoClose: 10000,
      })
      return
  }
}
