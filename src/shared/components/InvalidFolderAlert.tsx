import { Alert } from '@mantine/core'
import { IconAlertCircle } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import type { InvalidFolderAlertProps } from './types'

export function InvalidFolderAlert({ reason, path }: InvalidFolderAlertProps) {
  const { t } = useTranslation('onboarding')
  const { t: tErrors } = useTranslation('errors')

  return (
    <Alert
      color="red"
      icon={<IconAlertCircle size={20} stroke={1.8} />}
      title={t(($) => $.invalid.title)}
      role="alert"
    >
      {t(($) => $.invalid.body, { path, reason: tErrors(($) => $.folderProblems[reason]) })}
    </Alert>
  )
}
