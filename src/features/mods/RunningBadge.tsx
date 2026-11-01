import { Badge } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import type { RunningBadgeProps } from './types'

export function RunningBadge({ running }: RunningBadgeProps) {
  const { t } = useTranslation('game')

  return (
    <Badge
      color={running === 'running' ? 'green' : 'gray'}
      variant="light"
      size="lg"
      aria-live="polite"
      data-testid="running-badge"
    >
      {t(($) => $.running.short[running])}
    </Badge>
  )
}
