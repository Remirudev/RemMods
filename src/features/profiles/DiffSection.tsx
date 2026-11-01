import { Stack, Text } from '@mantine/core'
import type { DiffSectionProps } from './types'

export function DiffSection({ title, count, children }: DiffSectionProps) {
  if (count === 0) return null
  return (
    <Stack gap={4}>
      <Text size="sm" fw={600}>
        {title}
      </Text>
      <Stack gap={2} pl="sm">
        {children}
      </Stack>
    </Stack>
  )
}
