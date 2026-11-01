import { useTranslation } from 'react-i18next'
import { ProfileForm } from './ProfileForm'
import type { SaveProfileDialogProps } from './types'

export function SaveProfileDialog({ onClose }: SaveProfileDialogProps) {
  const { t } = useTranslation('profiles')

  return (
    <ProfileForm
      initial={{ name: '', description: '', mode: 'additive' }}
      submitLabel={t(($) => $.form.save)}
      buildSave={(values) => ({ kind: 'new', ...values })}
      onClose={onClose}
    />
  )
}
