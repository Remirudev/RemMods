import { Alert } from '@mantine/core'
import { IconInfoCircle } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'

export function OriginalAppNotice() {
  const { t } = useTranslation('game')

  return (
    <Alert
      color="blue"
      icon={<IconInfoCircle size={20} stroke={1.8} />}
      title={t(($) => $.originalApp.title)}
    >
      {t(($) => $.originalApp.body)}
    </Alert>
  )
}
