import { Button, Stack, Text, VisuallyHidden } from '@mantine/core'
import { useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import type { FolderBusyMessageProps } from './types'

function seconds(ms: number): number {
  return Math.max(0, Math.round(ms / 1000))
}

// The body of the folder-busy toast (WG-2). Its Cancel takes the focus when the wait starts, so
// Enter or Space cancels even while a dialog is open behind it.
export function FolderBusyMessage({
  info,
  cancelling,
  focusOnMount,
  onCancel,
}: FolderBusyMessageProps) {
  const { t } = useTranslation('mods')
  const cancelRef = useRef<HTMLButtonElement>(null)
  const folder = info.folder

  // biome-ignore lint/correctness/useExhaustiveDependencies: focus once, on mount only.
  useEffect(() => {
    if (focusOnMount) cancelRef.current?.focus()
  }, [])

  return (
    <Stack gap={6} data-testid="folder-busy">
      <VisuallyHidden aria-live="assertive">{t(($) => $.folderBusy.title)}</VisuallyHidden>
      <Text size="sm">
        {info.cancellable
          ? t(($) => $.folderBusy.body, { folder, limit: seconds(info.limitMs) })
          : t(($) => $.folderBusy.bodyUndo, { folder })}
      </Text>
      {info.holders.length > 0 && (
        <Text size="sm">{t(($) => $.folderBusy.holders, { names: info.holders.join(', ') })}</Text>
      )}
      {/* The seconds tick about once a second inside the alert; a screen reader must not re-read it. */}
      <Text size="xs" c="var(--qc-text-2)" aria-hidden="true">
        {t(($) => $.folderBusy.elapsed, { seconds: seconds(info.elapsedMs) })}
      </Text>
      {info.cancellable && (
        <Button
          ref={cancelRef}
          size="compact-sm"
          variant="default"
          disabled={cancelling}
          onClick={onCancel}
          style={{ alignSelf: 'flex-start' }}
        >
          {cancelling ? t(($) => $.folderBusy.cancelling) : t(($) => $.folderBusy.cancel)}
        </Button>
      )}
    </Stack>
  )
}
