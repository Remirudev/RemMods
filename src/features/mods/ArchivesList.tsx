import { Button, Code, Group, Paper, Stack, Text } from '@mantine/core'
import { IconDownload } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { formatSize } from '../../shared/format-size'
import { queueArchiveRow } from '../../shared/install-actions'
import { ListSection } from './ListSection'
import { RevealButton } from './RevealButton'
import type { ArchivesListProps } from './types'

export function ArchivesList({ archives }: ArchivesListProps) {
  const { t, i18n } = useTranslation('mods')
  const { t: tInstall } = useTranslation('install')

  return (
    <ListSection
      description={t(($) => $.archives.description)}
      empty={t(($) => $.archives.empty)}
      count={archives.length}
    >
      <Stack gap="xs">
        {archives.map((archive) => (
          <Paper key={archive.modRef} withBorder px="md" py="xs">
            <Group justify="space-between" wrap="nowrap" gap="md">
              <Code style={{ overflowWrap: 'anywhere' }}>
                <bdi>{archive.relPath}</bdi>
              </Code>
              <Group gap="sm" wrap="nowrap">
                <Text size="sm" c="var(--qc-text-2)">
                  {formatSize(archive.sizeBytes, i18n.language)}
                </Text>
                <Button
                  variant="default"
                  size="xs"
                  leftSection={<IconDownload size={16} stroke={1.8} />}
                  aria-label={tInstall(($) => $.row.installLabel, { name: archive.relPath })}
                  onClick={() => queueArchiveRow(archive.modRef, archive.relPath)}
                >
                  {tInstall(($) => $.row.install)}
                </Button>
                <RevealButton modRef={archive.modRef} label={t(($) => $.archives.reveal)} />
              </Group>
            </Group>
          </Paper>
        ))}
      </Stack>
    </ListSection>
  )
}
