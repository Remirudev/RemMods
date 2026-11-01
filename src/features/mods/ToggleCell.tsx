import { Box, Switch, Tooltip } from '@mantine/core'
import type { MouseEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { canToggle, isOn } from './toggle-model'
import type { ToggleCellProps } from './types'

export function ToggleCell({ row, onToggle }: ToggleCellProps) {
  const { t } = useTranslation('mods')
  const { t: tErrors } = useTranslation('errors')
  const on = isOn(row)
  const blocked = row.toggleBlock
  const label = on
    ? t(($) => $.toggle.disable, { name: row.name })
    : t(($) => $.toggle.enable, { name: row.name })

  // The cell swallows clicks so that using the switch never opens the details drawer or moves
  // the selection.
  return (
    <Tooltip
      label={blocked ? tErrors(($) => $.toggleBlocked[blocked]) : ''}
      disabled={blocked === null}
      position="right"
      withArrow
    >
      <Box
        component="span"
        data-testid="toggle-cell"
        display="inline-flex"
        onClick={(event: MouseEvent) => event.stopPropagation()}
      >
        <Switch
          size="sm"
          checked={on}
          disabled={!canToggle(row)}
          aria-label={label}
          onChange={() => onToggle([row], !on)}
        />
      </Box>
    </Tooltip>
  )
}
