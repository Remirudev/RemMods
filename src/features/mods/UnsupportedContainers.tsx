import { Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { containerStatusText } from './container-status'
import type { UnsupportedContainersProps } from './types'

export function UnsupportedContainers({ containers }: UnsupportedContainersProps) {
  const { t } = useTranslation('mods')

  return (
    <Stack gap={2}>
      {containers.map((container) => (
        <Text key={container.relPath} size="sm" c="var(--qc-text-2)">
          <bdi>{container.relPath}</bdi>
          {': '}
          {t(($) => $.content.unsupportedNote, { status: containerStatusText(container, t) })}
        </Text>
      ))}
    </Stack>
  )
}
