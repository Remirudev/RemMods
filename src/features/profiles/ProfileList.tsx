import { Stack } from '@mantine/core'
import { ProfileCard } from './ProfileCard'
import type { ProfileListProps } from './types'

export function ProfileList({ view, profiles }: ProfileListProps) {
  return (
    <Stack gap="sm">
      {profiles.map((summary) => (
        <ProfileCard key={summary.profile.id} summary={summary} view={view} />
      ))}
    </Stack>
  )
}
