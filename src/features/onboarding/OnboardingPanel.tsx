import { Alert, ScrollArea, Stack } from '@mantine/core'
import { IconAlertTriangle } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { DetectionNotes } from '../../shared/components/DetectionNotes'
import { GameLocationChooser } from '../../shared/components/GameLocationChooser'
import { PanelHeader } from '../../shared/components/PanelHeader'
import { useFolderChooser } from '../../shared/game/use-folder-chooser'
import type { OnboardingPanelProps } from './types'

export function OnboardingPanel({ status }: OnboardingPanelProps) {
  const { t } = useTranslation('onboarding')
  const chooser = useFolderChooser()
  const location = status.location

  return (
    <ScrollArea h="100%" data-testid="onboarding">
      <Stack gap="lg" pb="xl">
        <PanelHeader title={t(($) => $.title)} description={t(($) => $.description)} />
        {location.kind === 'missing' && (
          <Alert
            color="yellow"
            icon={<IconAlertTriangle size={20} stroke={1.8} />}
            title={t(($) => $.missing.title)}
          >
            {t(($) => $.missing.body, { path: location.path })}
          </Alert>
        )}
        <GameLocationChooser status={status} chooser={chooser} withActions />
        {status.notes.length > 0 && <DetectionNotes notes={status.notes} />}
      </Stack>
    </ScrollArea>
  )
}
