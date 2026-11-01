import { Stack } from '@mantine/core'
import { GameCandidateItem } from './GameCandidateItem'
import type { GameCandidateListProps } from './types'

export function GameCandidateList({ candidates, onUse, busyId }: GameCandidateListProps) {
  return (
    <Stack gap="sm">
      {candidates.map((candidate) => (
        <GameCandidateItem
          key={candidate.id}
          candidate={candidate}
          onUse={() => onUse(candidate.id)}
          busy={busyId === candidate.id}
        />
      ))}
    </Stack>
  )
}
