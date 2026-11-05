import { Alert, Center, Loader } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { splitProfiles } from './profile-label'
import { useProfiles } from './queries'
import { SnapshotList } from './SnapshotList'

// The automatic snapshots taken before every switch, newest first.
export function SnapshotsDialog() {
  const { t, i18n } = useTranslation('profiles')
  const profiles = useProfiles()
  const view = profiles.data ?? undefined

  if (profiles.isError) {
    return (
      <Alert color="red" role="alert">
        {t(($) => $.sidebar.loadError)}
      </Alert>
    )
  }
  if (!view) {
    return (
      <Center py="lg">
        <Loader size="sm" />
      </Center>
    )
  }
  const { snapshots } = splitProfiles(view.profiles, i18n.language)
  return <SnapshotList view={view} snapshots={snapshots} />
}
