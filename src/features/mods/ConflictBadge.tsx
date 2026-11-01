import { Badge } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import type { ModRowCellProps } from './types'

export function ConflictBadge({ row }: ModRowCellProps) {
  const { t } = useTranslation('mods')
  const count = row.conflictsWith.length
  if (count === 0) return null

  return (
    <Badge color="yellow" variant="light">
      {t(($) => $.badges.conflicts, { count })}
    </Badge>
  )
}
