import { Alert, Code, List, Stack, Text } from '@mantine/core'
import { IconAlertTriangle, IconInfoCircle, IconShieldX } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { groupIssues, issueTextKey } from './install-issues'
import type { IssueListProps } from './types'

const SEVERITY_STYLE = {
  block: { color: 'red', icon: <IconShieldX size={18} stroke={1.8} /> },
  warn: { color: 'yellow', icon: <IconAlertTriangle size={18} stroke={1.8} /> },
  info: { color: 'gray', icon: <IconInfoCircle size={18} stroke={1.8} /> },
} as const

export function IssueList({ issues }: IssueListProps) {
  const { t } = useTranslation('install')
  const groups = groupIssues(issues)
  if (groups.length === 0) return null

  return (
    <Stack gap="xs" data-testid="install-issues">
      {groups.map((group) => (
        <Alert
          key={group.severity}
          color={SEVERITY_STYLE[group.severity].color}
          icon={SEVERITY_STYLE[group.severity].icon}
          title={t(($) => $.issues.severity[group.severity])}
        >
          <List size="sm" spacing={4} listStyleType="none" ml={0}>
            {group.issues.map((issue) => (
              <List.Item key={`${issue.code}:${issue.path}`}>
                <Text size="sm" component="span">
                  {t(($) => $.issues[issueTextKey(issue.code)], { count: issue.count })}
                </Text>
                {issue.path !== '' && (
                  <>
                    {' '}
                    <Code>
                      <bdi>{issue.path}</bdi>
                    </Code>
                  </>
                )}
              </List.Item>
            ))}
          </List>
        </Alert>
      ))}
    </Stack>
  )
}
