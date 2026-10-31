import { ActionIcon, CopyButton, Group, Paper, Stack, Text, Tooltip } from '@mantine/core'
import { IconCheck, IconCopy } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { SectionHeading } from './SectionHeading'

const REPOSITORY_URL = 'https://github.com/Remirudev/RemMods'

export function AboutSection() {
  const { t } = useTranslation('settings')

  return (
    <Paper withBorder p="lg">
      <Stack gap="md">
        <SectionHeading
          title={t(($) => $.about.title)}
          description={t(($) => $.about.description)}
        />
        <Text size="sm">{t(($) => $.about.license)}</Text>
        <Text size="sm">{t(($) => $.about.credit)}</Text>
        <Group gap="xs" wrap="nowrap">
          <Text size="sm" fw={600}>
            {t(($) => $.about.source)}
          </Text>
          <Text size="sm" ff="monospace">
            {REPOSITORY_URL}
          </Text>
          <CopyButton value={REPOSITORY_URL}>
            {({ copied, copy }) => (
              <Tooltip label={copied ? t(($) => $.about.linkCopied) : t(($) => $.about.copyLink)}>
                <ActionIcon
                  variant="subtle"
                  color="gray"
                  aria-label={t(($) => $.about.copyLink)}
                  onClick={copy}
                >
                  {copied ? (
                    <IconCheck size={18} stroke={1.8} />
                  ) : (
                    <IconCopy size={18} stroke={1.8} />
                  )}
                </ActionIcon>
              </Tooltip>
            )}
          </CopyButton>
        </Group>
      </Stack>
    </Paper>
  )
}
