import { modals } from '@mantine/modals'
import type { ReactNode } from 'react'
import { i18n } from '../../lib/i18n'
import type { ApplyDirection, ModRow, Profile } from '../../lib/ipc/types'
import { GroupDialog } from './GroupDialog'
import { ProfileDiffModal } from './ProfileDiffModal'
import { profileTitle } from './profile-label'
import { SaveProfileDialog } from './SaveProfileDialog'
import { SnapshotsDialog } from './SnapshotsDialog'

function openWith(
  title: string,
  build: (close: () => void, modalId: string) => ReactNode,
  size?: string,
) {
  const modalId = crypto.randomUUID()
  modals.open({ modalId, title, size, children: build(() => modals.close(modalId), modalId) })
}

function profilesT() {
  return i18n.getFixedT(null, 'profiles')
}

export function unnamed(): string {
  return profilesT()(($) => $.unnamed)
}

export function titleOf(profile: Profile): string {
  return profileTitle(profile, unnamed())
}

// `profile` null: a new group. The dialog lists `rows` as they were when it opened.
export function openGroupDialog(profile: Profile | null, rows: readonly ModRow[]): void {
  const t = profilesT()
  openWith(
    profile === null
      ? t(($) => $.group.newTitle)
      : t(($) => $.group.editTitle, { name: titleOf(profile) }),
    (close, modalId) => (
      <GroupDialog profile={profile} rows={rows} onClose={close} modalId={modalId} />
    ),
    'xl',
  )
}

export function openSaveCurrent(): void {
  openWith(
    profilesT()(($) => $.form.saveTitle),
    (close) => <SaveProfileDialog onClose={close} />,
  )
}

export function openSnapshots(): void {
  openWith(
    profilesT()(($) => $.snapshots.dialogTitle),
    () => <SnapshotsDialog />,
    'lg',
  )
}

function switchTitle(profiles: readonly Profile[], direction: ApplyDirection): string {
  const t = profilesT()
  const [only] = profiles
  if (profiles.length === 1 && only) {
    const name = titleOf(only)
    return direction === 'on'
      ? t(($) => $.diff.titleOn, { name })
      : t(($) => $.diff.titleOff, { name })
  }
  const count = profiles.length
  return direction === 'on'
    ? t(($) => $.diff.titleOnMany, { count })
    : t(($) => $.diff.titleOffMany, { count })
}

// The change list before a switch of one group or several. `title` names what is applied when it
// is not a user group (a snapshot's label).
export function openSwitchDiff(
  profiles: readonly Profile[],
  direction: ApplyDirection,
  title?: string,
): void {
  const ids = profiles.map((profile) => profile.id)
  openWith(
    title ?? switchTitle(profiles, direction),
    (close, modalId) => (
      <ProfileDiffModal ids={ids} direction={direction} onClose={close} modalId={modalId} />
    ),
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
  modals.openConfirmModal({
    title,
    children: body,
    labels: { confirm, cancel: profilesT()(($) => $.form.cancel) },
    confirmProps: danger ? { color: 'red' } : undefined,
    onConfirm,
  })
}
