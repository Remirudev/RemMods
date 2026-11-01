import { Group, Loader, Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { describeError } from '../../lib/i18n/error-messages'
import { ContentHints } from './ContentHints'
import { ContentList } from './ContentList'
import { useModContents } from './queries'
import type { ContentSectionProps } from './types'
import { UnsupportedContainers } from './UnsupportedContainers'

export function ContentSection({ row }: ContentSectionProps) {
  const { t } = useTranslation('mods')
  const { content } = row
  const readCount = content.containers.filter((container) => container.status === 'read').length
  const unsupported = content.containers.filter((container) => container.status !== 'read')
  const contents = useModContents(readCount > 0 ? row.modRef : null)

  if (content.containers.length === 0) {
    return <Text size="sm">{t(($) => $.content.noUtoc)}</Text>
  }

  return (
    <Stack gap="sm">
      {readCount > 0 && (
        <>
          <Text size="sm">
            {t(($) => $.content.summary, { count: content.assets, containers: readCount })}
          </Text>
          <ContentHints content={content} />
          {content.internalOverlaps > 0 && (
            <Text size="sm" c="var(--qc-text-2)">
              {t(($) => $.content.internalOverlaps, { count: content.internalOverlaps })}
            </Text>
          )}
          {content.sharedIdentical > 0 && (
            <Text size="sm" c="var(--qc-text-2)">
              {t(($) => $.content.sharedIdentical, { count: content.sharedIdentical })}
            </Text>
          )}
        </>
      )}
      {unsupported.length > 0 && <UnsupportedContainers containers={unsupported} />}
      {readCount > 0 && contents.isPending && (
        <Group gap="xs">
          <Loader size="xs" />
          <Text size="sm">{t(($) => $.drawer.filesLoading)}</Text>
        </Group>
      )}
      {contents.isError && (
        <Text size="sm" c="red" role="alert">
          {describeError(contents.error).message}
        </Text>
      )}
      {contents.data && (
        <>
          {contents.data.base !== 'complete' && (
            <Text size="sm" c="var(--qc-text-2)">
              {t(($) => $.content.baseNote)}
            </Text>
          )}
          <ContentList entries={contents.data.entries} />
          {contents.data.truncated && (
            <Text size="sm" c="var(--qc-text-2)">
              {t(($) => $.content.truncated, {
                shown: contents.data.entries.length,
                total: contents.data.total,
              })}
            </Text>
          )}
        </>
      )}
    </Stack>
  )
}
