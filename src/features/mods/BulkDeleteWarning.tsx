import { Group, ScrollArea, Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { formatSize } from '../../shared/format-size'
import type { BulkDeleteWarningProps } from './types'

// The rows a permanent delete would remove, each with its size, and the line that says so.
export function BulkDeleteWarning({ heading, rows, footer }: BulkDeleteWarningProps) {
  const { t, i18n } = useTranslation('mods')
  const sizeOf = (size: number | null) =>
    formatSize(size, i18n.language) ?? t(($) => $.size.unknown)

  return (
    <Stack gap="xs" data-testid="bulk-delete-warning">
      {heading && <Text size="sm">{heading}</Text>}
      <ScrollArea.Autosize mah={200}>
        <Stack gap={4}>
          {rows.map((row) => (
            <Group key={row.key} gap="xs" wrap="nowrap" justify="space-between">
              <Text size="sm" truncate>
                <bdi>{row.name}</bdi>
                {row.note && (
                  <Text span size="xs" c="var(--qc-text-2)">
                    {' · '}
                    {row.note}
                  </Text>
                )}
              </Text>
              <Text size="sm" c="var(--qc-text-2)" style={{ flexShrink: 0 }}>
                {sizeOf(row.size)}
              </Text>
            </Group>
          ))}
        </Stack>
      </ScrollArea.Autosize>
      <Text size="sm">{footer}</Text>
      <Text size="sm" fw={600} c="red">
        {t(($) => $.delete.final.irreversible)}
      </Text>
    </Stack>
  )
}
