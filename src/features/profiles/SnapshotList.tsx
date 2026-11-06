import { Button, Group, Paper, Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { profileTitle, snapshotTarget } from './profile-label'
import { confirmProfile, openSwitchDiff } from './profile-modals'
import type { SnapshotListProps } from './types'
import { useDeleteProfile } from './use-profile-mutations'

export function SnapshotList({ view, snapshots }: SnapshotListProps) {
  const { t, i18n } = useTranslation('profiles')
  const remove = useDeleteProfile()
  const all = view.profiles.map((summary) => summary.profile)

  if (snapshots.length === 0) {
    return (
      <Text size="sm" c="var(--qc-text-2)">
        {t(($) => $.snapshots.empty)}
      </Text>
    )
  }

  const labelOf = (snapshot: (typeof all)[number]) => {
    const date = new Date(snapshot.created).toLocaleString(i18n.language)
    const target = snapshotTarget(snapshot, all)
    return target === null
      ? t(($) => $.snapshots.labelDeleted, { date })
      : t(($) => $.snapshots.label, {
          name: profileTitle(
            target,
            t(($) => $.unnamed),
          ),
          date,
        })
  }

  return (
    <Stack gap="xs">
      <Text size="sm" c="var(--qc-text-2)">
        {t(($) => $.snapshots.hint)}
      </Text>
      {snapshots.map(({ profile }) => {
        const label = labelOf(profile)
        return (
          <Paper key={profile.id} withBorder px="md" py="xs" data-testid="snapshot-item">
            <Group justify="space-between" wrap="nowrap" gap="md">
              <Text size="sm">
                <bdi>{label}</bdi>
              </Text>
              <Group gap="xs" wrap="nowrap">
                <Button
                  size="compact-md"
                  disabled={view.readOnly}
                  onClick={() => openSwitchDiff([profile], 'on', label)}
                >
                  {t(($) => $.snapshots.restore)}
                </Button>
                <Button
                  size="compact-md"
                  variant="default"
                  color="red"
                  disabled={view.readOnly}
                  onClick={() =>
                    confirmProfile(
                      t(($) => $.snapshots.deleteTitle),
                      t(($) => $.snapshots.deleteBody),
                      t(($) => $.row.delete),
                      () => remove.mutate(profile.id),
                      true,
                    )
                  }
                >
                  {t(($) => $.row.delete)}
                </Button>
              </Group>
            </Group>
          </Paper>
        )
      })}
    </Stack>
  )
}
