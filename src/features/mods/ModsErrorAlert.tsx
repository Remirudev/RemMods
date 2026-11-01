import { Alert, Button, Group, Stack, Text } from '@mantine/core'
import { IconAlertCircle } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { describeError } from '../../lib/i18n/error-messages'
import { useFolderChooser } from '../../shared/game/use-folder-chooser'
import type { ModsErrorAlertProps } from './types'

export function ModsErrorAlert({ error, onRetry }: ModsErrorAlertProps) {
  const { t } = useTranslation('mods')
  const { t: tSettings } = useTranslation('settings')
  const { t: tOnboarding } = useTranslation('onboarding')
  const chooser = useFolderChooser()
  const { message } = describeError(error)

  return (
    <Alert
      color="red"
      icon={<IconAlertCircle size={20} stroke={1.8} />}
      title={t(($) => $.error.title)}
      role="alert"
    >
      <Stack gap="sm" align="flex-start">
        <Text size="sm">{message}</Text>
        <Group gap="sm">
          <Button variant="default" onClick={onRetry}>
            {t(($) => $.error.retry)}
          </Button>
          <Button variant="default" loading={chooser.picking} onClick={chooser.pick}>
            {tSettings(($) => $.game.change)}
          </Button>
        </Group>
        {chooser.cancelled && <Text size="sm">{tOnboarding(($) => $.choose.cancelled)}</Text>}
      </Stack>
    </Alert>
  )
}
