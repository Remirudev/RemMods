import { Alert, Stack, Text } from '@mantine/core'
import { IconInfoCircle } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { CopyLinkRow } from './CopyLinkRow'
import { RELEASES_URL } from './links'
import type { UpdateKindNoticeProps } from './types'

export function UpdateKindNotice({ notice }: UpdateKindNoticeProps) {
  const { t } = useTranslation('settings')
  const text =
    notice.notice === 'package'
      ? t(($) => $.updates.notices.package, { format: notice.format ?? '' })
      : t(($) => $.updates.notices[notice.notice])

  return (
    <Alert color="blue" icon={<IconInfoCircle size={20} stroke={1.8} />}>
      <Stack gap="xs">
        <Text size="sm">{text}</Text>
        {notice.notice !== 'aur' && (
          <CopyLinkRow label={t(($) => $.updates.releases)} url={RELEASES_URL} />
        )}
      </Stack>
    </Alert>
  )
}
