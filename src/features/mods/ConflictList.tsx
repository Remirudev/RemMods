import { Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import type { ConflictListProps } from './types'

export function ConflictList({ conflicts }: ConflictListProps) {
  const { t, i18n } = useTranslation('mods')
  const list = new Intl.ListFormat(i18n.language, { type: 'conjunction' })

  if (conflicts.length === 0) return <Text size="sm">{t(($) => $.drawer.conflictsNone)}</Text>

  return (
    <Stack gap="xs">
      {conflicts.map((conflict) => (
        <Stack key={conflict.modRef} gap={0}>
          <Text size="sm" fw={600}>
            <bdi>{conflict.name}</bdi>
          </Text>
          <Text size="sm" c="var(--qc-text-2)">
            <bdi>{t(($) => $.drawer.conflictPaks, { paks: list.format(conflict.paks) })}</bdi>
          </Text>
        </Stack>
      ))}
    </Stack>
  )
}
