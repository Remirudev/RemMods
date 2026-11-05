import { Group, ScrollArea, Stack, Text, ThemeIcon } from '@mantine/core'
import { IconAlertTriangle, IconCheck, IconMinus, IconX } from '@tabler/icons-react'
import type { TFunction } from 'i18next'
import { useTranslation } from 'react-i18next'
import { describeAppError } from '../../lib/i18n/error-messages'
import type { BulkRow } from '../../lib/ipc/types'
import { formatSize } from '../../shared/format-size'
import { trashPlaceWord } from '../../shared/trash-place'
import { subjectOf, unavailableSize } from './bulk-model'
import { manageBlock } from './manage-model'
import type { BulkResultListProps, BulkRowStatus, ManageSubject } from './types'

const ICON = { size: 'sm', variant: 'light', radius: 'xl' } as const

// An unavailable row the matrix does not let RemMods delete says so instead of offering it.
function unavailableText(
  t: TFunction<'mods'>,
  tErrors: TFunction<'errors'>,
  row: BulkRow,
  subject: ManageSubject | undefined,
): string {
  const result = row.result
  if (result.kind !== 'unavailable') return ''
  const block = subject ? manageBlock(subject.kind, 'delete') : null
  if (block !== null) {
    const reason = tErrors(($) => $.manageBlocked.reason[block])
    return t(($) => $.bulk.results.notDeletable, { reason })
  }
  const trashReason = result.reason
  const reason = t(($) => $.trash.unavailable[trashReason], { place: trashPlaceWord() })
  return t(($) => $.bulk.results.unavailable, { reason })
}

function statusOf(
  t: TFunction<'mods'>,
  tErrors: TFunction<'errors'>,
  row: BulkRow,
  subject: ManageSubject | undefined,
): BulkRowStatus {
  const result = row.result
  switch (result.kind) {
    case 'trashed':
      return {
        icon: <IconCheck size={14} stroke={2} />,
        color: 'green',
        text:
          result.trash === 'recycleBin'
            ? t(($) => $.bulk.results.recycled)
            : t(($) => $.bulk.results.trashed),
      }
    case 'deleted':
      return result.leftover === null
        ? {
            icon: <IconCheck size={14} stroke={2} />,
            color: 'green',
            text: t(($) => $.bulk.results.deleted),
          }
        : {
            icon: <IconAlertTriangle size={14} stroke={2} />,
            color: 'yellow',
            text: t(($) => $.bulk.results.leftover, { path: result.leftover }),
          }
    case 'unavailable':
      return {
        icon: <IconAlertTriangle size={14} stroke={2} />,
        color: 'yellow',
        text: unavailableText(t, tErrors, row, subject),
      }
    case 'failed':
      // Earlier rows of the call may already be gone, so this says only this row is unchanged.
      if (result.error.kind === 'busyCancelled')
        return {
          icon: <IconMinus size={14} stroke={2} />,
          color: 'gray',
          text: t(($) => $.bulk.results.cancelled),
        }
      return {
        icon: <IconX size={14} stroke={2} />,
        color: 'red',
        text: t(($) => $.bulk.results.failed, { message: describeAppError(result.error).message }),
      }
    case 'notRun':
      return {
        icon: <IconMinus size={14} stroke={2} />,
        color: 'gray',
        text: t(($) => $.bulk.results.notRun),
      }
  }
}

// One line per row of a bulk call, in the order the backend ran them.
export function BulkResultList({ rows, subjects }: BulkResultListProps) {
  const { t, i18n } = useTranslation('mods')
  const { t: tErrors } = useTranslation('errors')

  return (
    <ScrollArea.Autosize mah={320}>
      <Stack gap={6} data-testid="bulk-results">
        {rows.map((row) => {
          const subject = subjectOf(row, subjects)
          const status = statusOf(t, tErrors, row, subject)
          const size = unavailableSize(row, subject)
          return (
            <Group
              key={`${row.target.modRef}|${row.target.key}`}
              gap="xs"
              wrap="nowrap"
              align="flex-start"
              data-testid="bulk-result-row"
            >
              <ThemeIcon {...ICON} color={status.color}>
                {status.icon}
              </ThemeIcon>
              <Stack gap={0} miw={0}>
                <Text size="sm" fw={600}>
                  <bdi>{row.name}</bdi>
                </Text>
                <Text size="xs" c="var(--qc-text-2)">
                  {status.text}
                  {row.result.kind === 'unavailable' &&
                    ` · ${formatSize(size, i18n.language) ?? t(($) => $.size.unknown)}`}
                </Text>
              </Stack>
            </Group>
          )
        })}
      </Stack>
    </ScrollArea.Autosize>
  )
}
