import { Alert, Button, Center, Loader, Stack } from '@mantine/core'
import { IconBookmarks, IconPlus } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { EmptyState } from '../../shared/components/EmptyState'
import { PanelHeader } from '../../shared/components/PanelHeader'
import { ProfileList } from './ProfileList'
import { splitProfiles } from './profile-label'
import { openSaveProfile } from './profile-modals'
import { useProfiles } from './queries'
import { SnapshotList } from './SnapshotList'
import { TransferMenu } from './TransferMenu'

export function ProfilesPanel() {
  const { t: tShell, i18n } = useTranslation('shell')
  const { t } = useTranslation('profiles')
  const profiles = useProfiles()
  const view = profiles.data ?? undefined
  const { user, snapshots } = splitProfiles(view?.profiles ?? [], i18n.language)

  return (
    <Stack gap="lg">
      <PanelHeader
        title={tShell(($) => $.panels.profiles.title)}
        description={tShell(($) => $.panels.profiles.description)}
        actions={
          <>
            <TransferMenu readOnly={view?.readOnly ?? true} />
            <Button
              leftSection={<IconPlus size={18} stroke={1.8} />}
              disabled={!view || view.readOnly}
              onClick={openSaveProfile}
            >
              {t(($) => $.panel.save)}
            </Button>
          </>
        }
      />
      {view?.readOnly && (
        <Alert color="yellow" role="status">
          {t(($) => $.panel.readOnly)}
        </Alert>
      )}
      {view?.problem && (
        <Alert color="red" role="alert">
          {t(($) => $.panel.problem[view.problem ?? 'unreadable'])}
        </Alert>
      )}
      {profiles.isError && (
        <Alert color="red" role="alert">
          {t(($) => $.panel.loadError)}
        </Alert>
      )}
      {!view && !profiles.isError && (
        <Center py="lg">
          <Loader size="md" />
        </Center>
      )}
      {view && user.length === 0 && snapshots.length === 0 && (
        <EmptyState
          icon={<IconBookmarks size={28} stroke={1.6} />}
          title={tShell(($) => $.panels.profiles.emptyTitle)}
          description={tShell(($) => $.panels.profiles.emptyDescription)}
        />
      )}
      {view && user.length > 0 && <ProfileList view={view} profiles={user} />}
      {view && <SnapshotList view={view} snapshots={snapshots} />}
    </Stack>
  )
}
