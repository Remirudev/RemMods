import { Progress, Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { formatSize } from '../../shared/format-size'
import type { UpdateProgressViewProps } from './types'
import { downloadPercent } from './update-model'

export function UpdateProgressView({ progress }: UpdateProgressViewProps) {
  const { t, i18n } = useTranslation('settings')
  const current = progress ?? { kind: 'downloading', downloaded: 0, total: null }
  const percent = downloadPercent(current)

  let label: string
  switch (current.kind) {
    case 'downloading':
      label =
        percent === null
          ? t(($) => $.updates.progress.downloadingSize, {
              size: formatSize(current.downloaded, i18n.language) ?? '',
            })
          : t(($) => $.updates.progress.downloading, { percent })
      break
    case 'installing':
      label = t(($) => $.updates.progress.installing)
      break
    case 'restarting':
      label = t(($) => $.updates.progress.restarting)
      break
  }

  return (
    <Stack gap="md" data-testid="update-progress">
      <Text size="sm">{label}</Text>
      <Progress value={percent ?? 100} animated={percent === null} aria-label={label} />
    </Stack>
  )
}
