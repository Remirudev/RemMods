import { Group, Stack, Text, Title } from '@mantine/core'
import type { PanelHeaderProps } from './types'

export function PanelHeader({ title, description, actions }: PanelHeaderProps) {
  return (
    <Group justify="space-between" align="flex-end" wrap="nowrap" gap="lg">
      <Stack gap={2}>
        <Title order={2} fz="h3">
          {title}
        </Title>
        {description && (
          <Text size="sm" c="var(--qc-text-2)">
            {description}
          </Text>
        )}
      </Stack>
      {actions && <Group gap="sm">{actions}</Group>}
    </Group>
  )
}
