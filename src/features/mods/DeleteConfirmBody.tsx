import { Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { formatSize } from '../../shared/format-size'
import { ConfirmButtons } from './ConfirmButtons'
import type { DeleteConfirmBodyProps } from './types'

const DOUBLE_ENTER_GUARD_MS = 1500

// The second, final confirm of a permanent delete.
export function DeleteConfirmBody({
  name,
  size,
  originalApp,
  onCancel,
  onConfirm,
}: DeleteConfirmBodyProps) {
  const { t, i18n } = useTranslation('mods')
  const sizeText = formatSize(size, i18n.language) ?? t(($) => $.size.unknown)

  return (
    <Stack gap="xs">
      <Text size="sm">{t(($) => $.delete.final.body, { name, size: sizeText })}</Text>
      <Text size="sm" fw={600} c="red">
        {t(($) => $.delete.final.irreversible)}
      </Text>
      {originalApp && <Text size="sm">{t(($) => $.confirm.originalAppRunning)}</Text>}
      <ConfirmButtons
        danger
        delayMs={DOUBLE_ENTER_GUARD_MS}
        cancelLabel={t(($) => $.delete.cancel)}
        confirmLabel={t(($) => $.delete.final.confirm)}
        onCancel={onCancel}
        onConfirm={onConfirm}
      />
    </Stack>
  )
}
