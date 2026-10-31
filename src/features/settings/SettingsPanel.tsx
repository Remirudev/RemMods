import { Stack } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { PanelHeader } from '../../shared/components/PanelHeader'
import { AboutSection } from './AboutSection'
import { AppearanceSection } from './AppearanceSection'
import { DiagnosticsSection } from './DiagnosticsSection'
import { SettingsAlerts } from './SettingsAlerts'

export function SettingsPanel() {
  const { t } = useTranslation('shell')

  return (
    <Stack gap="lg">
      <PanelHeader
        title={t(($) => $.panels.settings.title)}
        description={t(($) => $.panels.settings.description)}
      />
      <SettingsAlerts />
      <AppearanceSection />
      <DiagnosticsSection />
      <AboutSection />
    </Stack>
  )
}
