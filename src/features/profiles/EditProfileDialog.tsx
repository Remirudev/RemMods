import { useTranslation } from 'react-i18next'
import { ProfileForm } from './ProfileForm'
import type { EditProfileDialogProps } from './types'

export function EditProfileDialog({ profile, onClose }: EditProfileDialogProps) {
  const { t } = useTranslation('profiles')

  return (
    <ProfileForm
      initial={{ name: profile.name, description: profile.description, mode: profile.mode }}
      submitLabel={t(($) => $.form.saveChanges)}
      buildSave={(values) => ({ kind: 'edit', id: profile.id, ...values })}
      onClose={onClose}
    />
  )
}
