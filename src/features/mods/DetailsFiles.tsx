import { Badge, Group, Loader, Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { formatSize } from '../../shared/format-size'
import classes from './mods.module.css'
import { useModDetails } from './queries'
import type { DetailsFilesProps } from './types'

export function DetailsFiles({ modRef }: DetailsFilesProps) {
  const { t, i18n } = useTranslation('mods')
  const details = useModDetails(modRef)

  if (details.isPending) {
    return (
      <Group gap="xs">
        <Loader size="xs" />
        <Text size="sm">{t(($) => $.drawer.filesLoading)}</Text>
      </Group>
    )
  }
  if (details.isError) {
    return (
      <Text size="sm" c="red" role="alert">
        {t(($) => $.drawer.filesError)}
      </Text>
    )
  }

  const { files, truncated, otherFiles } = details.data
  return (
    <Stack gap="xs">
      <ul className={classes.fileList}>
        {files.map((file) => (
          <li key={`${file.relPath}|${file.kind}`}>
            <Group justify="space-between" wrap="nowrap" gap="sm" py={2}>
              <Text size="sm" ff="monospace" style={{ overflowWrap: 'anywhere' }}>
                <bdi>{file.relPath}</bdi>
              </Text>
              <Group gap="xs" wrap="nowrap">
                <Badge variant="light" color={file.live ? 'green' : 'gray'}>
                  {file.live ? t(($) => $.drawer.live) : t(($) => $.drawer.off)}
                </Badge>
                <Text size="xs" c="var(--qc-text-2)" miw={64} ta="right">
                  {formatSize(file.sizeBytes, i18n.language)}
                </Text>
              </Group>
            </Group>
          </li>
        ))}
      </ul>
      {truncated && (
        <Text size="sm" c="var(--qc-text-2)">
          {t(($) => $.drawer.filesTruncated, { count: files.length })}
        </Text>
      )}
      {otherFiles > 0 && (
        <Text size="sm" c="var(--qc-text-2)">
          {t(($) => $.drawer.otherFiles, { count: otherFiles })}
        </Text>
      )}
    </Stack>
  )
}
