import { Alert, Stack } from '@mantine/core'
import { IconAlertTriangle, IconInfoCircle } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { describeError } from '../../lib/i18n/error-messages'
import { useSettings } from './queries'

export function SettingsAlerts() {
  const { t } = useTranslation('settings')
  const settings = useSettings()

  if (settings.isError) {
    return (
      <Alert
        color="red"
        title={t(($) => $.alerts.loadFailedTitle)}
        icon={<IconAlertTriangle size={20} stroke={1.8} />}
      >
        {describeError(settings.error).message}
      </Alert>
    )
  }

  const data = settings.data
  if (!data || (!data.readOnly && !data.recovered)) return null

  return (
    <Stack gap="sm">
      {data.readOnly && (
        <Alert
          color="yellow"
          title={t(($) => $.alerts.readOnlyTitle)}
          icon={<IconAlertTriangle size={20} stroke={1.8} />}
        >
          {t(($) => $.alerts.readOnlyBody)}
        </Alert>
      )}
      {data.recovered && (
        <Alert
          color="blue"
          title={t(($) => $.alerts.recoveredTitle)}
          icon={<IconInfoCircle size={20} stroke={1.8} />}
        >
          {t(($) => $.alerts.recoveredBody)}
        </Alert>
      )}
    </Stack>
  )
}
