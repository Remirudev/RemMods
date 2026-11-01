import { Alert } from '@mantine/core'
import { IconInfoCircle } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { useWatchStatus } from './queries'

export function AutoRefreshBanner() {
  const { t } = useTranslation('mods')
  const state = useWatchStatus().data?.state
  if (state !== 'limitReached' && state !== 'failed') return null

  return (
    <Alert color="yellow" icon={<IconInfoCircle size={20} stroke={1.8} />} role="status">
      {state === 'limitReached' ? t(($) => $.watch.limitReached) : t(($) => $.watch.failed)}
    </Alert>
  )
}
