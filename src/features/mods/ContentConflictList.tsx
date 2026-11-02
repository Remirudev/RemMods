import { Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import type { ContentConflictListProps } from './types'

// Every link, whatever the slot; the mod list counts only those with clothing (`outfit`).
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
          {conflict.outfit > 0 && (
            <Text size="xs" c="orange">
              {t(($) => $.contentConflicts.outfit, { files: conflict.outfit })}
            </Text>
          )}
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
        {t(($) => $.contentConflicts.countRule)}
      </Text>
      <Text size="sm" c="var(--qc-text-2)">
        {t(($) => $.contentConflicts.winner)}
      </Text>
    </Stack>
  )
}
