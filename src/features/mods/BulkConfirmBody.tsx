import { List, Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { ConfirmButtons } from './ConfirmButtons'
import type { BulkConfirmBodyProps } from './types'

// The first confirm of a bulk action: what happens, the first names and how many more.
export function BulkConfirmBody({
  lines,
  names,
  more,
  confirmLabel,
  danger,
  onCancel,
  onConfirm,
}: BulkConfirmBodyProps) {
  const { t } = useTranslation('mods')
  const [first, ...rest] = lines

  return (
    <Stack gap="xs">
      {first && <Text size="sm">{first}</Text>}
      <List size="sm" spacing={2}>
        {names.map((entry) => (
          <List.Item key={entry.key}>
            <bdi>{entry.name}</bdi>
          </List.Item>
        ))}
        {more > 0 && <List.Item>{t(($) => $.bulk.trashConfirm.more, { count: more })}</List.Item>}
      </List>
      {rest.map((line) => (
        <Text key={line} size="sm">
          {line}
        </Text>
      ))}
      <ConfirmButtons
        danger={danger}
        cancelLabel={t(($) => $.delete.cancel)}
        confirmLabel={confirmLabel}
        onCancel={onCancel}
        onConfirm={onConfirm}
      />
    </Stack>
  )
}
