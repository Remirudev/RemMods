import { Alert, Button, Group, Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { describeError } from '../../lib/i18n/error-messages'
import { isIpcError } from '../../lib/ipc/ipc-error'
import type { InstallFailureProps } from './types'

// A split or password-protected archive cannot be read here, but its folder can be installed.
function suggestsFolder(error: unknown): boolean {
  if (!isIpcError(error) || error.appError.kind !== 'archiveFailed') return false
  const reason = error.appError.reason
  return reason === 'multiVolume' || reason === 'encrypted'
}

export function InstallFailure({
  error,
  canGoBack,
  remaining,
  onBack,
  onClose,
  onStop,
}: InstallFailureProps) {
  const { t } = useTranslation('install')
  const { title, message } = describeError(error)

  return (
    <Stack gap="md" data-testid="install-failure">
      <Alert color="red" title={title}>
        <Stack gap={4}>
          <Text size="sm">{message}</Text>
          {suggestsFolder(error) && <Text size="sm">{t(($) => $.failure.hints.extractFirst)}</Text>}
        </Stack>
      </Alert>
      <Group justify="flex-end">
        {canGoBack && (
          <Button variant="default" onClick={onBack}>
            {t(($) => $.actions.back)}
          </Button>
        )}
        <Button variant={remaining > 0 ? 'default' : 'filled'} onClick={onStop}>
          {t(($) => $.actions.close)}
        </Button>
        {remaining > 0 && <Button onClick={onClose}>{t(($) => $.actions.skip)}</Button>}
      </Group>
    </Stack>
  )
}
