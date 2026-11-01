import { Badge, Code, Group, Paper, Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { ListSection } from './ListSection'
import { RevealButton } from './RevealButton'
import type { UnmanagedListProps } from './types'

export function UnmanagedList({ entries }: UnmanagedListProps) {
  const { t, i18n } = useTranslation('mods')
  const list = new Intl.ListFormat(i18n.language, { type: 'conjunction' })

  return (
    <ListSection
      description={t(($) => $.unmanaged.description)}
      empty={t(($) => $.unmanaged.empty)}
      count={entries.length}
    >
      <Stack gap="xs">
        {entries.map((entry) => (
          <Paper key={entry.modRef} withBorder px="md" py="xs">
            <Group justify="space-between" wrap="nowrap" gap="md" align="flex-start">
              <Stack gap={4} miw={0}>
                <Group gap="xs">
                  <Badge variant="light" color="gray">
                    {t(($) => $.unmanaged.kinds[entry.kind])}
                  </Badge>
                  <Text size="sm" c="var(--qc-text-2)">
                    {entry.kind === 'linkedFolder'
                      ? t(($) => $.unmanaged.notInspected)
                      : t(($) => $.unmanaged.liveFiles, { count: entry.liveFiles })}
                  </Text>
                </Group>
                <Code style={{ overflowWrap: 'anywhere' }}>
                  <bdi>{entry.relPath}</bdi>
                </Code>
                {entry.copyOf.length > 0 && (
                  <Text size="sm">
                    <bdi>{t(($) => $.unmanaged.copyOf, { mods: list.format(entry.copyOf) })}</bdi>
                  </Text>
                )}
              </Stack>
              <RevealButton modRef={entry.modRef} label={t(($) => $.unmanaged.reveal)} />
            </Group>
          </Paper>
        ))}
      </Stack>
    </ListSection>
  )
}
