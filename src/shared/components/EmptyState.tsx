import { EmptyState as MantineEmptyState, Paper } from '@mantine/core'
import type { EmptyStateProps } from './types'

export function EmptyState({ icon, title, description }: EmptyStateProps) {
  return (
    <Paper withBorder p="xl">
      <MantineEmptyState variant="light" icon={icon} title={title} description={description} />
    </Paper>
  )
}
