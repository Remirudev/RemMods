import { Button, Group, Paper, Stack, Text } from '@mantine/core'
import { IconLicense } from '@tabler/icons-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { CopyLinkRow } from './CopyLinkRow'
import { REPOSITORY_URL } from './links'
import { NoticesModal } from './NoticesModal'
import { SectionHeading } from './SectionHeading'

export function AboutSection() {
  const { t } = useTranslation('settings')
  const [noticesOpen, setNoticesOpen] = useState(false)

  return (
    <Paper withBorder p="lg">
      <Stack gap="md">
        <SectionHeading
          title={t(($) => $.about.title)}
          description={t(($) => $.about.description)}
        />
        <Text size="sm">{t(($) => $.about.license)}</Text>
        <Text size="sm">{t(($) => $.about.credit)}</Text>
        <CopyLinkRow label={t(($) => $.about.source)} url={REPOSITORY_URL} />
        <Group>
          <Button
            variant="default"
            leftSection={<IconLicense size={18} stroke={1.8} />}
            onClick={() => setNoticesOpen(true)}
          >
            {t(($) => $.about.notices.open)}
          </Button>
        </Group>
      </Stack>
      <NoticesModal opened={noticesOpen} onClose={() => setNoticesOpen(false)} />
    </Paper>
  )
}
