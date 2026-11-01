import { Button, Collapse, Group, Paper, Stack, Text, UnstyledButton } from '@mantine/core'
import { IconChevronDown, IconChevronRight } from '@tabler/icons-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { profileTitle, snapshotTarget } from './profile-label'
import { confirmProfile, openProfileDiff } from './profile-modals'
import type { SnapshotListProps } from './types'
import { useDeleteProfile } from './use-profile-mutations'

export function SnapshotList({ view, snapshots }: SnapshotListProps) {
  const { t, i18n } = useTranslation('profiles')
  const [open, setOpen] = useState(false)
  const remove = useDeleteProfile()
  const all = view.profiles.map((summary) => summary.profile)
  if (snapshots.length === 0) return null

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
      <UnstyledButton onClick={() => setOpen((current) => !current)} aria-expanded={open}>
        <Group gap={6}>
          {open ? (
            <IconChevronDown size={16} stroke={1.8} />
          ) : (
            <IconChevronRight size={16} stroke={1.8} />
          )}
          <Text size="sm" fw={600}>
            {t(($) => $.snapshots.title, { count: snapshots.length })}
          </Text>
        </Group>
      </UnstyledButton>
      <Collapse expanded={open}>
        <Stack gap="xs">
          {snapshots.map(({ profile }) => {
            const label = labelOf(profile)
            return (
              <Paper key={profile.id} withBorder px="md" py="xs" data-testid="snapshot-item">
                <Group justify="space-between" wrap="nowrap" gap="md">
                  <Text size="sm">
                    <bdi>{label}</bdi>
                  </Text>
                  <Group gap="xs">
                    <Button
                      size="compact-md"
                      disabled={view.readOnly}
                      onClick={() => openProfileDiff(profile, label)}
                    >
                      {t(($) => $.card.apply)}
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
                          t(($) => $.card.delete),
                          () => remove.mutate(profile.id),
                          true,
                        )
                      }
                    >
                      {t(($) => $.card.delete)}
                    </Button>
                  </Group>
                </Group>
              </Paper>
            )
          })}
        </Stack>
      </Collapse>
    </Stack>
  )
}
