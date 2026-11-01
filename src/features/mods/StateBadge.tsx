import { Badge, Tooltip } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { displayState } from './display-state'
import type { ModDisplayState, ModRowCellProps } from './types'

const STATE_COLOR: Record<ModDisplayState, string> = {
  enabled: 'green',
  partial: 'yellow',
  disabledLoaded: 'orange',
  disabled: 'gray',
  noPakFiles: 'gray',
  unknown: 'gray',
}

export function StateBadge({ row }: ModRowCellProps) {
  const { t } = useTranslation('mods')
  const state = displayState(row)

  return (
    <Tooltip label={t(($) => $.stateHints[state])} multiline maw={300}>
      <Badge color={STATE_COLOR[state]} variant="light">
        {t(($) => $.states[state])}
      </Badge>
    </Tooltip>
  )
}
