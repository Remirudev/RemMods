import { Stack, Title } from '@mantine/core'
import type { DrawerSectionProps } from './types'

export function DrawerSection({ title, children }: DrawerSectionProps) {
  return (
    <Stack gap="xs">
      <Title order={4} fz="sm" tt="uppercase" c="var(--qc-text-2)">
        {title}
      </Title>
      {children}
    </Stack>
  )
}
