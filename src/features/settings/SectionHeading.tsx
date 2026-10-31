import { Stack, Text, Title } from '@mantine/core'
import type { SectionHeadingProps } from './types'

export function SectionHeading({ title, description }: SectionHeadingProps) {
  return (
    <Stack gap={2}>
      <Title order={3} fz="h4">
        {title}
      </Title>
      <Text size="sm" c="var(--qc-text-2)">
        {description}
      </Text>
    </Stack>
  )
}
