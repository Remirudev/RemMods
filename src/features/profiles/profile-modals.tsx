import { modals } from '@mantine/modals'
import type { ReactNode } from 'react'
import { i18n } from '../../lib/i18n'
import type { Profile, ProfilesView } from '../../lib/ipc/types'
import { queryClient } from '../../lib/query/client'
import { queryKeys } from '../../shared/query-keys'
import { EditProfileDialog } from './EditProfileDialog'
import { ProfileDiffModal } from './ProfileDiffModal'
import { profileTitle } from './profile-label'
import { SaveProfileDialog } from './SaveProfileDialog'

function openWith(
  title: string,
  build: (close: () => void, modalId: string) => ReactNode,
  size?: string,
) {
  const modalId = crypto.randomUUID()
  modals.open({ modalId, title, size, children: build(() => modals.close(modalId), modalId) })
}

export function openSaveProfile(): void {
  const t = i18n.getFixedT(null, 'profiles')
  openWith(
    t(($) => $.form.saveTitle),
    (close) => <SaveProfileDialog onClose={close} />,
  )
}

export function openEditProfile(profile: Profile): void {
  const t = i18n.getFixedT(null, 'profiles')
  openWith(
    t(($) => $.form.editTitle),
    (close) => <EditProfileDialog profile={profile} onClose={close} />,
  )
}

export function openProfileDiff(profile: Profile, title: string): void {
  openWith(
    title,
    (close, modalId) => <ProfileDiffModal profile={profile} onClose={close} modalId={modalId} />,
    'lg',
  )
}

export function confirmProfile(
  title: string,
  body: string,
  confirm: string,
  onConfirm: () => void,
  danger = false,
): void {
  const t = i18n.getFixedT(null, 'profiles')
  modals.openConfirmModal({
    title,
    children: body,
    labels: { confirm, cancel: t(($) => $.form.cancel) },
    confirmProps: danger ? { color: 'red' } : undefined,
    onConfirm,
  })
}

export function unnamed(): string {
  return i18n.getFixedT(null, 'profiles')(($) => $.unnamed)
}

export function titleOf(profile: Profile): string {
  return profileTitle(profile, unnamed())
}

// Quick-apply from the Mods header: the same diff modal and title as the panel's Apply.
export function openProfileApplyById(profileId: string): boolean {
  const view = queryClient.getQueryData<ProfilesView>(queryKeys.profiles)
  const profile = view?.profiles.find((summary) => summary.profile.id === profileId)?.profile
  if (!profile) return false
  const t = i18n.getFixedT(null, 'profiles')
  openProfileDiff(
    profile,
    t(($) => $.diff.title, { name: titleOf(profile) }),
  )
  return true
}
