import { Button } from '@mantine/core'
import { IconAlertTriangle } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { useModsUiStore } from './mods-ui-store'
import type { ConflictsChipProps } from './types'

export function ConflictsChip({ rows }: ConflictsChipProps) {
  const { t } = useTranslation('mods')
  const toggle = useModsUiStore((state) => state.toggleConflictsView)
  const active = useModsUiStore((state) => state.tab === 'mods' && state.filter.conflictsOnly)
  const count = rows.filter((row) => row.conflictsWith.length > 0).length
  if (count === 0) return null

  return (
    <Button
      color="yellow"
      variant={active ? 'filled' : 'light'}
      size="compact-md"
      leftSection={<IconAlertTriangle size={16} stroke={1.8} />}
      aria-pressed={active}
      onClick={toggle}
    >
      {t(($) => $.header.conflicts, { count })}
    </Button>
  )
}
