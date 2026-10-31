import { Button, Center, Code, Group, Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import type { ErrorFallbackProps } from './types'

export function ErrorFallback({ area, error, onReset }: ErrorFallbackProps) {
  const { t } = useTranslation('common')
  const title = area
    ? t(($) => $.errorBoundary.title, { area })
    : t(($) => $.errorBoundary.titleGeneric)

  return (
    <Center h="100%" p="xl">
      <Stack gap="sm" maw={520} align="center">
        <Text fw={600} size="lg">
          {title}
        </Text>
        <Text size="sm" c="dimmed" ta="center">
          {t(($) => $.errorBoundary.hint)}
        </Text>
        <Code block maw="100%" style={{ whiteSpace: 'pre-wrap' }}>
          {error.message}
        </Code>
        <Group>
          <Button variant="default" onClick={onReset}>
            {t(($) => $.actions.tryAgain)}
          </Button>
          <Button variant="subtle" color="gray" onClick={() => window.location.reload()}>
            {t(($) => $.actions.reload)}
          </Button>
        </Group>
      </Stack>
    </Center>
  )
}
