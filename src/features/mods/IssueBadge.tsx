import { Badge } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { SEVERITY_COLOR, worstSeverity } from './issue-severity'
import type { ModRowCellProps } from './types'

export function IssueBadge({ row }: ModRowCellProps) {
  const { t } = useTranslation('mods')
  const severity = worstSeverity(row.issues)
  if (severity === null) return null

  return (
    <Badge color={SEVERITY_COLOR[severity]} variant="light" title={t(($) => $.severity[severity])}>
      {t(($) => $.badges.issues, { count: row.issues.length })}
    </Badge>
  )
}
