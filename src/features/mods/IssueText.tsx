import { Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { issueMessage } from './issue-message'
import type { IssueTextProps } from './types'

export function IssueText({ issue, root }: IssueTextProps) {
  const { t, i18n } = useTranslation('mods')
  const { t: tErrors } = useTranslation('errors')

  return (
    <Text size="sm">
      <bdi>{issueMessage(issue, root, t, tErrors, i18n.language)}</bdi>
    </Text>
  )
}
