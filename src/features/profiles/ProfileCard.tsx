import { Badge, Button, Group, Paper, Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { profileTitle, relativeTime } from './profile-label'
import { confirmProfile, openEditProfile, openProfileDiff } from './profile-modals'
import type { ProfileCardProps } from './types'
import { useDeleteProfile, useSaveProfile } from './use-profile-mutations'

export function ProfileCard({ summary, view }: ProfileCardProps) {
  const { t, i18n } = useTranslation('profiles')
  const { profile } = summary
  const save = useSaveProfile()
  const remove = useDeleteProfile()
  const title = profileTitle(
    profile,
    t(($) => $.unnamed),
  )
  const unsaved = view.lastApplied?.profileId === profile.id && !summary.matchesCurrent
  const locked = view.readOnly

  const confirmOverwrite = () =>
    confirmProfile(
      t(($) => $.card.updateTitle),
      t(($) => $.card.updateBody, { name: title }),
      t(($) => $.card.update),
      () => save.mutate({ kind: 'overwrite', id: profile.id }),
    )
  const confirmDelete = () =>
    confirmProfile(
      t(($) => $.card.deleteTitle),
      t(($) => $.card.deleteBody, { name: title }),
      t(($) => $.card.delete),
      () => remove.mutate(profile.id),
      true,
    )

  return (
    <Paper withBorder p="md" data-testid="profile-card">
      <Stack gap="xs">
        <Group justify="space-between" align="flex-start" wrap="nowrap" gap="md">
          <Stack gap={2} miw={0}>
            <Group gap="xs">
              <Text fw={600}>
                <bdi>{title}</bdi>
              </Text>
              <Badge variant="light" size="sm">
                {t(($) => $.mode[profile.mode].label)}
              </Badge>
              {profile.source.kind === 'otherManager' && (
                <Badge variant="outline" size="sm" color="gray">
                  {t(($) => $.card.fromOtherManager)}
                </Badge>
              )}
              {profile.source.kind === 'import' && (
                <Badge variant="outline" size="sm" color="gray">
                  {t(($) => $.card.imported)}
                </Badge>
              )}
              {unsaved && (
                <Badge variant="light" size="sm" color="yellow">
                  {t(($) => $.card.unsaved)}
                </Badge>
              )}
            </Group>
            {profile.description !== '' && (
              <Text size="sm" c="var(--qc-text-2)">
                <bdi>{profile.description}</bdi>
              </Text>
            )}
            <Text size="xs" c="var(--qc-text-2)">
              {t(($) => $.card.counts, { enabled: summary.enabled, disabled: summary.disabled })}
              {summary.missing > 0 && ` · ${t(($) => $.card.missing, { count: summary.missing })}`}
              {profile.lastApplied !== null &&
                ` · ${t(($) => $.card.lastApplied, {
                  when: relativeTime(profile.lastApplied, i18n.language, Date.now()),
                })}`}
            </Text>
          </Stack>
          <Group gap="xs" wrap="wrap" justify="flex-end">
            <Button
              size="compact-md"
              disabled={locked}
              onClick={() =>
                openProfileDiff(
                  profile,
                  t(($) => $.diff.title, { name: title }),
                )
              }
            >
              {t(($) => $.card.apply)}
            </Button>
            <Button
              size="compact-md"
              variant="default"
              disabled={locked}
              onClick={confirmOverwrite}
            >
              {t(($) => $.card.update)}
            </Button>
            <Button
              size="compact-md"
              variant="default"
              disabled={locked}
              onClick={() => openEditProfile(profile)}
            >
              {t(($) => $.card.edit)}
            </Button>
            <Button
              size="compact-md"
              variant="default"
              color="red"
              disabled={locked}
              onClick={confirmDelete}
            >
              {t(($) => $.card.delete)}
            </Button>
          </Group>
        </Group>
      </Stack>
    </Paper>
  )
}
