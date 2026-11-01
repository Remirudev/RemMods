import { ScrollArea, Stack, Text } from '@mantine/core'
import type { ListSectionProps } from './types'

export function ListSection({ description, empty, count, children }: ListSectionProps) {
  return (
    <ScrollArea flex={1} mih={0}>
      <Stack gap="md" pb="md">
        <Text size="sm" c="var(--qc-text-2)">
          {description}
        </Text>
        {count === 0 ? <Text size="sm">{empty}</Text> : children}
      </Stack>
    </ScrollArea>
  )
}
