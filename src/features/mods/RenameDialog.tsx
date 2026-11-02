import { Button, Group, Stack, Text, TextInput } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ipc } from '../../lib/ipc'
import { isIpcError } from '../../lib/ipc/ipc-error'
import { FS_SCOPE, routeFsError } from '../../shared/fs-error-route'
import { storeModsSnapshot } from '../../shared/mods-cache'
import { useModalLock } from '../../shared/use-modal-lock'
import { checkName } from './name-check'
import type { NameIssueCode, RenameDialogProps } from './types'

const CORE_CODES: ReadonlySet<string> = new Set<NameIssueCode>([
  'empty',
  'dot_name',
  'too_long',
  'forbidden_char',
  'control_char',
  'bidi_control',
  'trailing_dot_or_space',
  'reserved_device_name',
  'disabled_prefix',
  'temp_name',
])

function issueFromError(error: unknown): NameIssueCode | 'invalid' | null {
  if (!isIpcError(error)) return null
  const appError = error.appError
  if (appError.kind === 'invalidName') {
    return CORE_CODES.has(appError.issue) ? (appError.issue as NameIssueCode) : 'invalid'
  }
  if (appError.kind === 'targetExists') return 'exists'
  if (appError.kind === 'invalidInput' && appError.field === 'name') return 'unchanged'
  return null
}

export function RenameDialog({
  target,
  currentName,
  initialName,
  siblings,
  originalApp,
  onClose,
  modalId,
}: RenameDialogProps) {
  const { t } = useTranslation('mods')
  const client = useQueryClient()
  const [name, setName] = useState(initialName)
  const [coreIssue, setCoreIssue] = useState<NameIssueCode | 'invalid' | null>(null)
  const inputRef = useRef<HTMLInputElement>(null)
  const submitted = useRef(false)
  const check = checkName(name, currentName, siblings)
  const issue = check ?? coreIssue

  useEffect(() => {
    inputRef.current?.select()
  }, [])

  const rename = useMutation({
    scope: FS_SCOPE,
    meta: { suppressErrorToast: true },
    mutationFn: (applyWhileRunning: boolean) =>
      ipc.modsRename({ target, newName: name, applyWhileRunning }),
    onSuccess: (outcome) => {
      storeModsSnapshot(client, outcome.snapshot)
      notifications.show({
        message: t(($) => $.rename.done, { from: currentName, to: name }),
        autoClose: 6000,
      })
      if (outcome.rekey.failed) {
        notifications.show({
          color: 'yellow',
          message: t(($) => $.rename.rekeyFailed),
          autoClose: 10000,
        })
      }
      onClose()
    },
    onError: (error) => {
      submitted.current = false
      const code = issueFromError(error)
      if (code) setCoreIssue(code)
      else routeFsError(error, (applyWhileRunning) => rename.mutate(applyWhileRunning))
    },
  })

  useModalLock(modalId, rename.isPending)

  const submit = (event: FormEvent) => {
    event.preventDefault()
    if (check !== null || submitted.current) return
    submitted.current = true
    rename.mutate(false)
  }

  return (
    <form onSubmit={submit}>
      <Stack gap="sm">
        <TextInput
          ref={inputRef}
          data-autofocus
          label={t(($) => $.rename.label)}
          value={name}
          error={issue && issue !== 'unchanged' ? t(($) => $.nameIssue[issue]) : undefined}
          onChange={(event) => {
            setName(event.currentTarget.value)
            setCoreIssue(null)
          }}
        />
        {originalApp && <Text size="sm">{t(($) => $.confirm.originalAppRunning)}</Text>}
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose} disabled={rename.isPending}>
            {t(($) => $.rename.cancel)}
          </Button>
          <Button type="submit" disabled={issue !== null} loading={rename.isPending}>
            {t(($) => $.rename.submit)}
          </Button>
        </Group>
      </Stack>
    </form>
  )
}
