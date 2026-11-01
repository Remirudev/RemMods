import { Badge, Group, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { ContentConflictBadge } from './ContentConflictBadge'
import type { ModRowCellProps } from './types'

export function NameCell({ row }: ModRowCellProps) {
  const { t } = useTranslation('mods')

  return (
    <Group gap="xs" wrap="nowrap">
      <Text size="sm" truncate="end" title={row.name}>
        <bdi>{row.name}</bdi>
      </Text>
      {row.ambiguous && (
        <Badge color="red" variant="light" size="sm">
          {t(($) => $.badges.ambiguous)}
        </Badge>
      )}
      <ContentConflictBadge row={row} />
    </Group>
  )
}
