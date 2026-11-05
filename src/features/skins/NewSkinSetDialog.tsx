import { Button, Group, Stack, Text, TextInput } from '@mantine/core'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isIpcError } from '../../lib/ipc/ipc-error'
import { routeFsError } from '../../shared/fs-error-route'
import { useModalLock } from '../../shared/use-modal-lock'
import { useSaveNewSkinSet } from './queries'
import type { NewSkinSetDialogProps } from './types'

const NAME_MAX_CHARS = 100

type Problem = 'taken' | 'invalid' | null

function problemOf(error: unknown): Problem {
  if (!isIpcError(error)) return null
  const { appError } = error
  if (appError.kind === 'skinSetNameTaken') return 'taken'
  return appError.kind === 'invalidInput' ? 'invalid' : null
}

// Saves the game's skins, or with `copy` keeps an automatic copy instead.
export function NewSkinSetDialog({ modalId, copy, onClose }: NewSkinSetDialogProps) {
  const { t } = useTranslation('skins')
  const [name, setName] = useState('')
  const [problem, setProblem] = useState<Problem>(null)
  const save = useSaveNewSkinSet()
  useModalLock(modalId, save.isPending)
  const trimmed = name.trim()
  const unusable = trimmed === '' || [...trimmed].length > NAME_MAX_CHARS

  const submit = (event: FormEvent) => {
    event.preventDefault()
    if (unusable || save.isPending) return
    save.mutate(
      { name: trimmed, copy },
      {
        onSuccess: onClose,
        onError: (error) => {
          const known = problemOf(error)
          if (known) setProblem(known)
          else routeFsError(error)
        },
      },
    )
  }

  return (
    <form onSubmit={submit}>
      <Stack gap="md" data-testid="new-skin-set">
        <Text size="sm" c="var(--qc-text-2)">
          {copy ? t(($) => $.keepCopy.hint) : t(($) => $.newSet.hint)}
        </Text>
        <TextInput
          data-autofocus
          label={t(($) => $.newSet.name)}
          value={name}
          error={problem ? t(($) => $.problem[problem]) : undefined}
          onChange={(event) => {
            setName(event.currentTarget.value)
            setProblem(null)
          }}
        />
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose} disabled={save.isPending}>
            {t(($) => $.newSet.cancel)}
          </Button>
          <Button type="submit" loading={save.isPending} disabled={unusable}>
            {t(($) => $.newSet.create)}
          </Button>
        </Group>
      </Stack>
    </form>
  )
}
