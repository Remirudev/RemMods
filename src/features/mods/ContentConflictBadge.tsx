import { Badge, Tooltip } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import type { ModRowCellProps } from './types'

export function ContentConflictBadge({ row }: ModRowCellProps) {
  const { t, i18n } = useTranslation('mods')
  const count = row.contentConflicts.length
  if (count === 0) return null

  const names = new Intl.ListFormat(i18n.language, { type: 'conjunction' }).format(
    row.contentConflicts.map((link) => link.name),
  )
  return (
    <Tooltip label={t(($) => $.badges.contentConflictsTooltip, { names })} multiline maw={320}>
      <Badge color="orange" variant="light" size="sm" style={{ flexShrink: 0 }}>
        {t(($) => $.badges.contentConflicts, { count })}
      </Badge>
    </Tooltip>
  )
}
