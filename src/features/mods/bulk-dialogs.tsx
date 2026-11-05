import { i18n } from '../../lib/i18n'
import { trashPlaceWord } from '../../shared/trash-place'
import { BulkConfirmBody } from './BulkConfirmBody'
import { BulkFinalBody } from './BulkFinalBody'
import { BulkResultsBody } from './BulkResultsBody'
import { totalSize } from './bulk-model'
import { openSettled, sizeText } from './flow-dialogs'
import type { BulkDialog, BulkResults, ManageSubject } from './types'

const MAX_NAMES = 10

function firstNames(subjects: readonly ManageSubject[]) {
  return subjects.slice(0, MAX_NAMES).map((subject) => ({
    key: `${subject.target.modRef}|${subject.target.key}`,
    name: subject.name,
  }))
}

function extraLines(skipped: number): string[] {
  const t = i18n.getFixedT(null, 'mods')
  return skipped > 0 ? [t(($) => $.bulk.skippedByMatrix, { count: skipped })] : []
}

// Opens the confirm for one step of a bulk flow; true only on an explicit confirm.
export function askBulkDialog(dialog: BulkDialog): Promise<boolean> {
  const t = i18n.getFixedT(null, 'mods')
  const place = trashPlaceWord()
  switch (dialog.kind) {
    case 'bulkTrash': {
      const { subjects, skipped } = dialog
      const count = subjects.length
      const size = sizeText(totalSize(subjects.map((subject) => subject.sizeBytes)))
      return openSettled(
        t(($) => $.bulk.trashConfirm.title, { place }),
        false,
        (settle) => (
          <BulkConfirmBody
            lines={[
              t(($) => $.bulk.trashConfirm.body, { count, size, place }),
              ...extraLines(skipped),
            ]}
            names={firstNames(subjects)}
            more={Math.max(0, count - MAX_NAMES)}
            confirmLabel={t(($) => $.bulk.trashConfirm.confirm, { place })}
            danger={false}
            onCancel={() => settle(false)}
            onConfirm={() => settle(true)}
          />
        ),
      )
    }
    case 'bulkDeleteFirst': {
      const { subjects, skipped } = dialog
      const count = subjects.length
      return openSettled(
        t(($) => $.bulk.deleteFirst.title),
        false,
        (settle) => (
          <BulkConfirmBody
            lines={[t(($) => $.bulk.deleteFirst.body, { count, place }), ...extraLines(skipped)]}
            names={firstNames(subjects)}
            more={Math.max(0, count - MAX_NAMES)}
            confirmLabel={t(($) => $.bulk.deleteFirst.confirm)}
            danger={false}
            onCancel={() => settle(false)}
            onConfirm={() => settle(true)}
          />
        ),
      )
    }
    case 'bulkDeleteFinal':
      return openSettled(
        t(($) => $.bulk.deleteFinal.title),
        false,
        (settle) => (
          <BulkFinalBody
            rows={dialog.rows}
            onCancel={() => settle(false)}
            onConfirm={() => settle(true)}
          />
        ),
        'lg',
      )
  }
}

export function showBulkResults(results: BulkResults): Promise<'deleteUnavailable' | 'close'> {
  const t = i18n.getFixedT(null, 'mods')
  return openSettled<'deleteUnavailable' | 'close'>(
    t(($) => $.bulk.results.title),
    'close',
    (settle) => (
      <BulkResultsBody
        results={results}
        onClose={() => settle('close')}
        onDelete={() => settle('deleteUnavailable')}
      />
    ),
    'lg',
  )
}
