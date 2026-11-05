import { Button, Group, Stack } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { formatSize } from '../../shared/format-size'
import { trashPlaceWord } from '../../shared/trash-place'
import { BulkDeleteWarning } from './BulkDeleteWarning'
import { BulkResultList } from './BulkResultList'
import { deletableUnavailable, subjectOf, totalSize, unavailableSize } from './bulk-model'
import { ConfirmButtons } from './ConfirmButtons'
import type { BulkResultsBodyProps } from './types'

// What a bulk call did, row by row. After a trash, the rows the Trash refused and RemMods may
// delete are offered for a permanent delete: that button is the second confirmation.
export function BulkResultsBody({ results, onClose, onDelete }: BulkResultsBodyProps) {
  const { t, i18n } = useTranslation('mods')
  const deletable =
    results.action === 'trash' ? deletableUnavailable(results.rows, results.subjects) : []
  const rows = deletable.map((row) => {
    const reason = row.result.kind === 'unavailable' ? row.result.reason : 'notSupported'
    return {
      key: `${row.target.modRef}|${row.target.key}`,
      name: row.name,
      size: unavailableSize(row, subjectOf(row, results.subjects)),
      note: t(($) => $.trash.unavailable[reason], { place: trashPlaceWord() }),
    }
  })
  const total =
    formatSize(totalSize(rows.map((row) => row.size)), i18n.language) ?? t(($) => $.size.unknown)

  return (
    <Stack gap="sm">
      <BulkResultList rows={results.rows} subjects={results.subjects} />
      {rows.length > 0 ? (
        <>
          <BulkDeleteWarning
            rows={rows}
            footer={t(($) => $.bulk.results.deleteWarning, { count: rows.length, size: total })}
          />
          <ConfirmButtons
            danger
            cancelLabel={t(($) => $.bulk.results.close)}
            confirmLabel={t(($) => $.bulk.results.deleteUnavailable, { count: rows.length })}
            onCancel={onClose}
            onConfirm={onDelete}
          />
        </>
      ) : (
        <Group justify="flex-end" mt="md">
          <Button variant="default" onClick={onClose} data-autofocus>
            {t(($) => $.bulk.results.close)}
          </Button>
        </Group>
      )}
    </Stack>
  )
}
