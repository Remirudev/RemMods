import { Badge, Tooltip } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { outfitConflicts } from './content-conflicts'
import type { ModRowCellProps } from './types'

export function ContentConflictBadge({ row }: ModRowCellProps) {
  const { t, i18n } = useTranslation('mods')
  const counted = outfitConflicts(row)
  if (counted.length === 0) return null

  const names = new Intl.ListFormat(i18n.language, { type: 'conjunction' }).format(
    counted.map((link) => link.name),
  )
  return (
    <Tooltip label={t(($) => $.badges.contentConflictsTooltip, { names })} multiline maw={320}>
      <Badge color="orange" variant="light">
        {t(($) => $.badges.contentConflicts, { count: counted.length })}
      </Badge>
    </Tooltip>
  )
}
