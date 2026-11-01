import { Code, Paper, Stack, Text, UnstyledButton } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { IssueText } from './IssueText'
import { rootsWithProblems, rowsWithIssues } from './issue-count'
import { ListSection } from './ListSection'
import { useModsUiStore } from './mods-ui-store'
import type { IssuesListProps } from './types'

export function IssuesList({ rows, roots, leftovers }: IssuesListProps) {
  const { t } = useTranslation('mods')
  const { t: tErrors } = useTranslation('errors')
  const openDetails = useModsUiStore((state) => state.openDetails)
  const withIssues = rowsWithIssues(rows)
  const problemRoots = rootsWithProblems(roots)
  const rootMessages = problemRoots.flatMap((root) => {
    const messages: string[] = []
    if (root.caseVariant) {
      messages.push(t(($) => $.notices.caseVariantRoots, { names: root.name }))
    }
    const unreadable = root.unreadable
    if (unreadable) {
      const reason = tErrors(($) => $.ioClasses[unreadable.class])
      messages.push(t(($) => $.notices.unreadableRoot, { root: root.name, reason }))
    }
    return messages
  })

  return (
    <ListSection
      description={t(($) => $.issues.description)}
      empty={t(($) => $.issues.empty)}
      count={withIssues.length + rootMessages.length + leftovers.length}
    >
      <Stack gap="xs">
        {rootMessages.map((message) => (
          <Paper key={message} withBorder px="md" py="xs">
            <Text size="sm">
              <bdi>{message}</bdi>
            </Text>
          </Paper>
        ))}
        {withIssues.map((row) => (
          <Paper key={row.modRef} withBorder px="md" py="xs">
            <UnstyledButton w="100%" onClick={() => openDetails(row.modRef)}>
              <Stack gap={4}>
                <Text size="sm" fw={600}>
                  <bdi>{row.name}</bdi>
                </Text>
                {row.issues.map((issue) => (
                  <IssueText key={JSON.stringify(issue)} issue={issue} root={row.root} />
                ))}
              </Stack>
            </UnstyledButton>
          </Paper>
        ))}
        {leftovers.length > 0 && (
          <Paper withBorder px="md" py="xs">
            <Stack gap={4}>
              <Text size="sm" fw={600}>
                {t(($) => $.issues.leftovers, { count: leftovers.length })}
              </Text>
              {leftovers.map((leftover) => (
                <Code key={leftover.modRef} style={{ overflowWrap: 'anywhere' }}>
                  <bdi>{leftover.relPath}</bdi>
                </Code>
              ))}
            </Stack>
          </Paper>
        )}
      </Stack>
    </ListSection>
  )
}
