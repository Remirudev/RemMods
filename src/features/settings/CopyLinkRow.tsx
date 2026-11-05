import { ActionIcon, CopyButton, Group, Text, Tooltip } from '@mantine/core'
import { IconCheck, IconCopy } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import type { CopyLinkRowProps } from './types'

// Shown and copied only: RemMods opens no web links.
export function CopyLinkRow({ label, url }: CopyLinkRowProps) {
  const { t } = useTranslation('settings')

  return (
    <Group gap="xs" wrap="nowrap">
      <Text size="sm" fw={600}>
        {label}
      </Text>
      <Text size="sm" ff="monospace">
        {url}
      </Text>
      <CopyButton value={url}>
        {({ copied, copy }) => (
          <Tooltip label={copied ? t(($) => $.about.linkCopied) : t(($) => $.about.copyLink)}>
            <ActionIcon
              variant="subtle"
              color="gray"
              aria-label={t(($) => $.about.copyLink)}
              onClick={copy}
            >
              {copied ? <IconCheck size={18} stroke={1.8} /> : <IconCopy size={18} stroke={1.8} />}
            </ActionIcon>
          </Tooltip>
        )}
      </CopyButton>
    </Group>
  )
}
