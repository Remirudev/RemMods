import { Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { describeAppError } from '../../lib/i18n/error-messages'
import { formatSize } from '../../shared/format-size'
import { BulkDeleteWarning } from './BulkDeleteWarning'
import { eligibleRows, totalSize } from './bulk-model'
import { ConfirmButtons } from './ConfirmButtons'
import type { BulkFinalBodyProps } from './types'

const DOUBLE_ENTER_GUARD_MS = 1500

// The second, final confirm of a bulk permanent delete: exactly the rows the token covers.
export function BulkFinalBody({ rows, onCancel, onConfirm }: BulkFinalBodyProps) {
  const { t, i18n } = useTranslation('mods')
  const eligible = eligibleRows(rows)
  const refused = rows.filter((row) => row.refusal !== null)
  const total =
    formatSize(totalSize(eligible.map((row) => row.size)), i18n.language) ??
    t(($) => $.size.unknown)

  return (
    <Stack gap="xs">
      <BulkDeleteWarning
        heading={t(($) => $.bulk.deleteFinal.body)}
        rows={eligible.map((row) => ({
          key: `${row.target.modRef}|${row.target.key}`,
          name: row.name,
          size: row.size,
        }))}
        footer={t(($) => $.bulk.deleteFinal.total, { size: total })}
      />
      {refused.map((row) => (
        <Text key={`${row.target.modRef}|${row.target.key}`} size="sm" c="var(--qc-text-2)">
          <bdi>{row.name}</bdi>
          {' · '}
          {t(($) => $.bulk.deleteFinal.skipped, {
            reason: row.refusal ? describeAppError(row.refusal).message : '',
          })}
        </Text>
      ))}
      <ConfirmButtons
        danger
        delayMs={DOUBLE_ENTER_GUARD_MS}
        cancelLabel={t(($) => $.delete.cancel)}
        confirmLabel={t(($) => $.bulk.deleteFinal.confirm, { count: eligible.length })}
        onCancel={onCancel}
        onConfirm={onConfirm}
      />
    </Stack>
  )
}
