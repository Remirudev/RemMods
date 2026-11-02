import { Button, Group, Progress, Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { formatSize } from '../../shared/format-size'
import type { InstallProgressViewProps } from './types'

export function InstallProgressView({ progress, onCancel }: InstallProgressViewProps) {
  const { t, i18n } = useTranslation('install')
  const phase = progress?.phase ?? 'preparing'
  const total = progress?.totalBytes ?? 0
  const done = progress?.doneBytes ?? 0
  const percent = total > 0 ? Math.min(100, (done / total) * 100) : 0
  const cancellable = phase === 'preparing' || phase === 'extracting'

  return (
    <Stack gap="md" data-testid="install-progress">
      <Text size="sm">{t(($) => $.progress.phase[phase])}</Text>
      <Progress
        value={percent}
        animated={phase !== 'extracting'}
        aria-label={t(($) => $.progress.phase[phase])}
      />
      <Group justify="space-between">
        <Text size="xs" c="var(--qc-text-2)">
          {t(($) => $.progress.sizes, {
            done: formatSize(done, i18n.language) ?? '',
            total: formatSize(total, i18n.language) ?? '',
          })}
        </Text>
        <Text size="xs" c="var(--qc-text-2)">
          {t(($) => $.progress.files, {
            done: progress?.doneFiles ?? 0,
            total: progress?.totalFiles ?? 0,
          })}
        </Text>
      </Group>
      <Group justify="flex-end">
        <Button variant="default" disabled={!cancellable} onClick={onCancel}>
          {t(($) => $.actions.cancel)}
        </Button>
      </Group>
    </Stack>
  )
}
