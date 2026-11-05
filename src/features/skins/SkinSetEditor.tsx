import { Button, Group, Stack, Text, Textarea, TextInput } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconRestore, IconTrash } from '@tabler/icons-react'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isIpcError } from '../../lib/ipc/ipc-error'
import { formatSize } from '../../shared/format-size'
import { routeFsError } from '../../shared/fs-error-route'
import { useModalLock } from '../../shared/use-modal-lock'
import { useDeleteSkinSet, useEditSkinSet, usePutBackSkinSet } from './queries'
import { ScreenshotList } from './ScreenshotList'
import { confirmDeleteSkinSet, confirmPutBack } from './skin-modals'
import type { SkinSetEditorProps } from './types'

const NOTES_MAX_CHARS = 4000

export function SkinSetEditor({ set, active, locked, modalId, onClose }: SkinSetEditorProps) {
  const { t, i18n } = useTranslation('skins')
  const [name, setName] = useState(set.name)
  const [notes, setNotes] = useState(set.notes)
  const [taken, setTaken] = useState(false)
  const edit = useEditSkinSet()
  const remove = useDeleteSkinSet(onClose)
  const putBack = usePutBackSkinSet(() =>
    notifications.show({ message: t(($) => $.putBack.done, { name: set.name }) }),
  )
  useModalLock(modalId, edit.isPending || remove.isPending || putBack.isPending)

  const trimmed = name.trim()
  const changed = trimmed !== set.name || notes !== set.notes
  const unusable = trimmed === '' || [...notes].length > NOTES_MAX_CHARS

  const submit = (event: FormEvent) => {
    event.preventDefault()
    if (!changed || unusable || edit.isPending) return
    edit.mutate(
      { id: set.id, name: trimmed, notes },
      {
        onError: (error) => {
          if (isIpcError(error) && error.appError.kind === 'skinSetNameTaken') setTaken(true)
          else routeFsError(error)
        },
      },
    )
  }

  return (
    <Stack gap="lg" data-testid="skin-set-dialog">
      <form onSubmit={submit}>
        <Stack gap="sm">
          <TextInput
            label={t(($) => $.dialog.name)}
            value={name}
            error={taken ? t(($) => $.problem.taken) : undefined}
            onChange={(event) => {
              setName(event.currentTarget.value)
              setTaken(false)
            }}
          />
          <Textarea
            label={t(($) => $.dialog.notes)}
            placeholder={t(($) => $.dialog.notesHint)}
            autosize
            minRows={2}
            maxRows={6}
            value={notes}
            onChange={(event) => setNotes(event.currentTarget.value)}
          />
          <Group justify="flex-end">
            <Button
              type="submit"
              variant="default"
              size="compact-sm"
              loading={edit.isPending}
              disabled={!changed || unusable}
            >
              {t(($) => $.dialog.save)}
            </Button>
          </Group>
        </Stack>
      </form>
      <ScreenshotList set={set} />
      <Group justify="space-between">
        <Text size="xs" c="var(--qc-text-2)">
          {t(($) => $.dialog.size, { size: formatSize(set.bytes, i18n.language) ?? '' })}
        </Text>
        <Group gap="xs">
          {active && (
            <Button
              size="compact-sm"
              variant="subtle"
              color="gray"
              leftSection={<IconRestore size={16} stroke={1.8} />}
              disabled={locked}
              loading={putBack.isPending}
              onClick={() => confirmPutBack(set, () => putBack.mutate(set.id))}
            >
              {t(($) => $.dialog.putBack)}
            </Button>
          )}
          <Button
            size="compact-sm"
            variant="subtle"
            color="red"
            leftSection={<IconTrash size={16} stroke={1.8} />}
            disabled={active}
            title={active ? t(($) => $.dialog.deleteActive) : undefined}
            loading={remove.isPending}
            onClick={() => confirmDeleteSkinSet(set, () => remove.mutate(set.id))}
          >
            {t(($) => $.dialog.delete)}
          </Button>
        </Group>
      </Group>
    </Stack>
  )
}
