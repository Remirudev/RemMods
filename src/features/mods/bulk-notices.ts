import { notifications } from '@mantine/notifications'
import { i18n } from '../../lib/i18n'
import { placeOfKind } from '../../shared/trash-place'
import type { BulkNotice } from './types'

export function showBulkNotice(notice: BulkNotice): void {
  const t = i18n.getFixedT(null, 'mods')
  switch (notice.kind) {
    case 'bulkTrashDone': {
      const count = notice.count
      const message =
        placeOfKind(notice.trash) === 'recycleBin'
          ? t(($) => $.bulk.done.recycle, { count })
          : t(($) => $.bulk.done.trash, { count })
      notifications.show({ message, autoClose: 6000 })
      return
    }
    case 'bulkDeleteDone':
      notifications.show({
        message: t(($) => $.bulk.done.delete, { count: notice.count }),
        autoClose: 6000,
      })
      return
    case 'bulkNothingEligible':
      notifications.show({ color: 'yellow', message: t(($) => $.bulk.nothingEligible) })
      return
    case 'deleteExpired':
      notifications.show({ color: 'yellow', message: t(($) => $.delete.expired), autoClose: 8000 })
      return
  }
}
