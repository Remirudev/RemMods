import { Alert, Button, Group, Stack, Textarea, TextInput } from '@mantine/core'
import { modals } from '@mantine/modals'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ipc } from '../../lib/ipc'
import { isIpcError } from '../../lib/ipc/ipc-error'
import type { ModInfoEdit } from '../../lib/ipc/types'
import { FS_SCOPE, routeFsError } from '../../shared/fs-error-route'
import { storeModsSnapshot } from '../../shared/mods-cache'
import { originalAppRunning } from './flow-deps'
import { fieldTooLong, isDirty, toEdit } from './metadata-form'
import { modInfoText } from './mod-info-text'
import type { MetadataEditorProps, ModInfoField } from './types'

const TEXT_FIELDS: readonly Exclude<ModInfoField, 'description'>[] = [
  'version',
  'category',
  'author',
]

type EditProblem = 'ambiguous' | 'tooLong' | null

function problemOf(error: unknown): EditProblem {
  if (!isIpcError(error)) return null
  const appError = error.appError
  if (appError.kind === 'modinfoAmbiguous') return 'ambiguous'
  if (appError.kind === 'invalidInput' && appError.field !== 'name') return 'tooLong'
  return null
}

export function MetadataEditor({ row, onDone }: MetadataEditorProps) {
  const { t } = useTranslation('mods')
  const client = useQueryClient()
  const original = toEdit(row.info)
  const [edit, setEdit] = useState<ModInfoEdit>(original)
  const [problem, setProblem] = useState<EditProblem>(null)
  const tooLong = fieldTooLong(edit)
  const target = { modRef: row.modRef, key: row.key }

  const save = useMutation({
    scope: FS_SCOPE,
    meta: { suppressErrorToast: true },
    mutationFn: () => ipc.modsSetMetadata({ target, info: edit }),
    onSuccess: (snapshot) => {
      storeModsSnapshot(client, snapshot)
      onDone()
    },
    onError: (error) => {
      const known = problemOf(error)
      if (known) setProblem(known)
      else routeFsError(error)
    },
  })

  const submit = () => {
    setProblem(null)
    if (!originalAppRunning(client)) {
      save.mutate()
      return
    }
    modals.openConfirmModal({
      title: t(($) => $.metadata.confirmTitle),
      children: t(($) => $.confirm.originalAppRunning),
      labels: { confirm: t(($) => $.metadata.save), cancel: t(($) => $.metadata.cancel) },
      onConfirm: () => save.mutate(),
    })
  }

  const placeholder = (field: ModInfoField) =>
    row.info.defaulted[field] ? modInfoText(row.info, field, t) : ''
  const label = (field: ModInfoField) => t(($) => $.drawer[field])
  const setField = (field: ModInfoField, value: string) => {
    setEdit((current) => ({ ...current, [field]: value }))
    setProblem(null)
  }

  return (
    <Stack gap="sm" data-testid="metadata-editor">
      {TEXT_FIELDS.map((field) => (
        <TextInput
          key={field}
          label={label(field)}
          placeholder={placeholder(field)}
          value={edit[field]}
          onChange={(event) => setField(field, event.currentTarget.value)}
        />
      ))}
      <Textarea
        autosize
        minRows={2}
        maxRows={8}
        label={label('description')}
        placeholder={placeholder('description')}
        value={edit.description}
        onChange={(event) => setField('description', event.currentTarget.value)}
      />
      {(problem === 'ambiguous' || tooLong !== null || problem === 'tooLong') && (
        <Alert color="red" role="alert">
          {problem === 'ambiguous'
            ? t(($) => $.metadata.ambiguous)
            : t(($) => $.metadata.tooLong, { max: 10000 })}
        </Alert>
      )}
      <Group gap="sm">
        <Button
          onClick={submit}
          loading={save.isPending}
          disabled={!isDirty(edit, original) || tooLong !== null}
        >
          {t(($) => $.metadata.save)}
        </Button>
        <Button variant="default" onClick={onDone}>
          {t(($) => $.metadata.cancel)}
        </Button>
      </Group>
    </Stack>
  )
}
