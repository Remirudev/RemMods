import { Stack, Text } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { formatSize } from '../../shared/format-size'
import { trashPlaceWord } from '../../shared/trash-place'
import { ConfirmButtons } from './ConfirmButtons'
import type { TrashUnavailableBodyProps } from './types'

// Shown when the system Trash cannot take the item: the only way on is a permanent delete.
export function TrashUnavailableBody({
  name,
  size,
  reason,
  otherManager,
  onCancel,
  onConfirm,
}: TrashUnavailableBodyProps) {
  const { t, i18n } = useTranslation('mods')
  const sizeText = formatSize(size, i18n.language) ?? t(($) => $.size.unknown)

  return (
    <Stack gap="xs">
      <Text size="sm">{t(($) => $.trash.unavailable[reason], { place: trashPlaceWord() })}</Text>
      <Text size="sm">{t(($) => $.trash.unavailable.warning, { name, size: sizeText })}</Text>
      <Text size="sm" fw={600} c="red">
        {t(($) => $.delete.final.irreversible)}
      </Text>
      {otherManager && <Text size="sm">{t(($) => $.confirm.otherManagerRunning)}</Text>}
      <ConfirmButtons
        danger
        cancelLabel={t(($) => $.delete.cancel)}
        confirmLabel={t(($) => $.trash.unavailable.confirm)}
        onCancel={onCancel}
        onConfirm={onConfirm}
      />
    </Stack>
  )
}
