import { Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import type { ContentConflictListProps } from './types'

export function ContentConflictList({ conflicts }: ContentConflictListProps) {
  const { t } = useTranslation('mods')

  return (
    <Stack gap="sm">
      {conflicts.map((conflict) => (
        <Stack key={conflict.modRef} gap={2}>
          <Text size="sm" fw={600}>
            <bdi>
              {t(($) => $.contentConflicts.item, { count: conflict.shared, name: conflict.name })}
            </bdi>
          </Text>
          {conflict.sample.map((path) => (
            <Text
              key={path}
              size="xs"
              c="var(--qc-text-2)"
              ff="monospace"
              style={{ overflowWrap: 'anywhere' }}
            >
              <bdi>{path}</bdi>
            </Text>
          ))}
        </Stack>
      ))}
      <Text size="sm" c="var(--qc-text-2)">
        {t(($) => $.contentConflicts.winner)}
      </Text>
    </Stack>
  )
}
