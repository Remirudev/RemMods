import { Button, Group, Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import type { RunningConfirmProps } from './types'

export function RunningConfirm({ onConfirm, onBack }: RunningConfirmProps) {
  const { t } = useTranslation('install')

  return (
    <Stack gap="md" data-testid="install-running-confirm">
      <Text fw={600}>{t(($) => $.running.question)}</Text>
      <Text size="sm">{t(($) => $.running.body)}</Text>
      <Group justify="flex-end">
        <Button variant="default" onClick={onBack}>
          {t(($) => $.actions.back)}
        </Button>
        <Button onClick={onConfirm}>{t(($) => $.running.confirm)}</Button>
      </Group>
    </Stack>
  )
}
