import { Stack } from '@mantine/core'
import { IconPackages } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { EmptyState } from '../../shared/components/EmptyState'
import { PanelHeader } from '../../shared/components/PanelHeader'

export function ModsPanel() {
  const { t } = useTranslation('shell')

  return (
    <Stack gap="lg">
      <PanelHeader
        title={t(($) => $.panels.mods.title)}
        description={t(($) => $.panels.mods.description)}
      />
      <EmptyState
        icon={<IconPackages size={28} stroke={1.6} />}
        title={t(($) => $.panels.mods.emptyTitle)}
        description={t(($) => $.panels.mods.emptyDescription)}
      />
    </Stack>
  )
}
