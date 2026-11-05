import { Button, Group } from '@mantine/core'
import { useEffect, useState } from 'react'
import type { ConfirmButtonsProps } from './types'

export function ConfirmButtons({
  cancelLabel,
  confirmLabel,
  danger = false,
  delayMs = 0,
  onCancel,
  onConfirm,
}: ConfirmButtonsProps) {
  const [ready, setReady] = useState(delayMs === 0)

  useEffect(() => {
    if (delayMs === 0) return
    const timer = setTimeout(() => setReady(true), delayMs)
    return () => clearTimeout(timer)
  }, [delayMs])

  return (
    <Group justify="flex-end" mt="md">
      <Button variant="default" onClick={onCancel} data-autofocus>
        {cancelLabel}
      </Button>
      <Button color={danger ? 'red' : undefined} disabled={!ready} onClick={onConfirm}>
        {confirmLabel}
      </Button>
    </Group>
  )
}
