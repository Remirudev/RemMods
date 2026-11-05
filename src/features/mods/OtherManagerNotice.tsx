import { Alert } from '@mantine/core'
import { IconInfoCircle } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'

export function OtherManagerNotice() {
  const { t } = useTranslation('game')

  return (
    <Alert
      color="blue"
      icon={<IconInfoCircle size={20} stroke={1.8} />}
      title={t(($) => $.otherManager.title)}
    >
      {t(($) => $.otherManager.body)}
    </Alert>
  )
}
