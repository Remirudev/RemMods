import { Stack, Text } from '@mantine/core'
import { modals } from '@mantine/modals'
import type { ReactNode } from 'react'
import { i18n } from '../../lib/i18n'
import { formatSize } from '../../shared/format-size'
import { trashPlaceWord } from '../../shared/trash-place'
import { DeleteConfirmBody } from './DeleteConfirmBody'
import { TrashUnavailableBody } from './TrashUnavailableBody'
import type { FlowDialog } from './types'

export function sizeText(size: number | null): string {
  const t = i18n.getFixedT(null, 'mods')
  return formatSize(size, i18n.language) ?? t(($) => $.size.unknown)
}

// Opens a modal whose body settles the answer; any other way to close it answers `fallback`.
export function openSettled<T>(
  title: string,
  fallback: T,
  body: (settle: (answer: T) => void) => ReactNode,
  size?: string,
): Promise<T> {
  return new Promise<T>((resolve) => {
    const modalId = crypto.randomUUID()
    const settle = (answer: T) => {
      resolve(answer)
      modals.close(modalId)
    }
    modals.open({ modalId, title, size, children: body(settle), onClose: () => resolve(fallback) })
  })
}

function openCustom(title: string, body: (settle: (answer: boolean) => void) => ReactNode) {
  return openSettled(title, false, body)
}

function openPlain(title: string, lines: string[], confirm: string, cancel: string, red: boolean) {
  return new Promise<boolean>((resolve) => {
    modals.openConfirmModal({
      title,
      children: (
        <Stack gap="xs">
          {lines.map((line) => (
            <Text key={line} size="sm">
              {line}
            </Text>
          ))}
        </Stack>
      ),
      labels: { confirm, cancel },
      confirmProps: red ? { color: 'red' } : undefined,
      onConfirm: () => resolve(true),
      onCancel: () => resolve(false),
      onClose: () => resolve(false),
    })
  })
}

// Opens the confirm for one step of a flow and resolves true only on an explicit confirm.
export function askFlowDialog(dialog: FlowDialog, originalApp: boolean): Promise<boolean> {
  const t = i18n.getFixedT(null, 'mods')
  const warning = originalApp ? [t(($) => $.confirm.originalAppRunning)] : []
  const place = trashPlaceWord()
  switch (dialog.kind) {
    case 'trash':
      return openPlain(
        t(($) => $.trash.confirm.title, { place }),
        [
          t(($) => $.trash.confirm.body, {
            name: dialog.subject.name,
            size: sizeText(dialog.subject.sizeBytes),
            place,
          }),
          ...warning,
        ],
        t(($) => $.trash.confirm.confirm, { place }),
        t(($) => $.delete.cancel),
        false,
      )
    case 'deleteFirst':
      return openPlain(
        t(($) => $.delete.confirm.title),
        [t(($) => $.delete.confirm.body, { name: dialog.subject.name, place }), ...warning],
        t(($) => $.delete.confirm.confirm),
        t(($) => $.delete.cancel),
        false,
      )
    case 'trashUnavailable':
      return openCustom(
        t(($) => $.trash.unavailable.title, { place }),
        (settle) => (
          <TrashUnavailableBody
            name={dialog.subject.name}
            size={dialog.size}
            reason={dialog.reason}
            originalApp={originalApp}
            onCancel={() => settle(false)}
            onConfirm={() => settle(true)}
          />
        ),
      )
    case 'deleteFinal':
      return openCustom(
        t(($) => $.delete.final.title, { name: dialog.name }),
        (settle) => (
          <DeleteConfirmBody
            name={dialog.name}
            size={dialog.size}
            originalApp={originalApp}
            onCancel={() => settle(false)}
            onConfirm={() => settle(true)}
          />
        ),
      )
  }
}
