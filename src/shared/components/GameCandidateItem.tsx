import { Badge, Button, Code, Group, Paper, Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import type { GameCandidateItemProps } from './types'

export function GameCandidateItem({ candidate, onUse, busy }: GameCandidateItemProps) {
  const { t } = useTranslation('game')
  const { t: tErrors } = useTranslation('errors')
  const problem = candidate.problem

  return (
    <Paper withBorder p="md" data-testid="game-candidate">
      <Group justify="space-between" align="center" wrap="nowrap" gap="md">
        <Stack gap={6} miw={0}>
          <Code style={{ overflowWrap: 'anywhere' }}>
            <bdi>{candidate.root}</bdi>
          </Code>
          <Group gap="xs">
            <Text size="sm" c="var(--qc-text-2)">
              {t(($) => $.steamKinds[candidate.steamKind])}
            </Text>
            <Text size="sm" c="var(--qc-text-2)">
              {candidate.buildId
                ? t(($) => $.candidate.build, { build: candidate.buildId })
                : t(($) => $.candidate.buildUnknown)}
            </Text>
            {!candidate.fullyInstalled && (
              <Badge color="yellow" variant="light">
                {t(($) => $.candidate.incomplete)}
              </Badge>
            )}
            {candidate.inUse && (
              <Badge color="green" variant="light">
                {t(($) => $.candidate.inUse)}
              </Badge>
            )}
          </Group>
          {problem && (
            <Text size="sm" c="red">
              {tErrors(($) => $.folderProblems[problem])}
            </Text>
          )}
        </Stack>
        <Button
          variant="default"
          disabled={problem !== null || candidate.inUse}
          loading={busy}
          onClick={onUse}
        >
          {t(($) => $.candidate.use)}
        </Button>
      </Group>
    </Paper>
  )
}
