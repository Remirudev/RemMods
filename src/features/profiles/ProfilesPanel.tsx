import { Stack } from '@mantine/core'
import { IconBookmarks } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { EmptyState } from '../../shared/components/EmptyState'
import { PanelHeader } from '../../shared/components/PanelHeader'

export function ProfilesPanel() {
  const { t } = useTranslation('shell')

  return (
    <Stack gap="lg">
      <PanelHeader
        title={t(($) => $.panels.profiles.title)}
        description={t(($) => $.panels.profiles.description)}
      />
      <EmptyState
        icon={<IconBookmarks size={28} stroke={1.6} />}
        title={t(($) => $.panels.profiles.emptyTitle)}
        description={t(($) => $.panels.profiles.emptyDescription)}
      />
    </Stack>
  )
}
