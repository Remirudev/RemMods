import { Button, Group, SegmentedControl, Stack, Text, Textarea, TextInput } from '@mantine/core'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isIpcError } from '../../lib/ipc/ipc-error'
import type { ProfileMode } from '../../lib/ipc/types'
import { showErrorToast } from '../../lib/query/error-toast'
import type { ProfileFormProps } from './types'
import { useSaveProfile } from './use-profile-mutations'

const NAME_MAX_CHARS = 100
const MODES: readonly ProfileMode[] = ['additive', 'exclusive']

type FormProblem = 'taken' | 'invalid' | null

function problemOf(error: unknown): FormProblem {
  if (!isIpcError(error)) return null
  if (error.appError.kind === 'profileNameTaken') return 'taken'
  if (error.appError.kind === 'invalidInput') return 'invalid'
  return null
}

export function ProfileForm({ initial, submitLabel, buildSave, onClose }: ProfileFormProps) {
  const { t } = useTranslation('profiles')
  const [values, setValues] = useState(initial)
  const [problem, setProblem] = useState<FormProblem>(null)
  const save = useSaveProfile()
  const name = values.name.trim()
  const lengthProblem = name === '' || [...name].length > NAME_MAX_CHARS
  const shown = problem ?? (lengthProblem && values.name !== '' ? 'invalid' : null)

  const submit = (event: FormEvent) => {
    event.preventDefault()
    if (lengthProblem || save.isPending) return
    save.mutate(buildSave({ ...values, name }), {
      onSuccess: onClose,
      onError: (error) => {
        const known = problemOf(error)
        if (known) setProblem(known)
        else showErrorToast(error)
      },
    })
  }

  return (
    <form onSubmit={submit}>
      <Stack gap="sm">
        <TextInput
          data-autofocus
          label={t(($) => $.form.name)}
          value={values.name}
          error={shown ? t(($) => $.form.problem[shown]) : undefined}
          onChange={(event) => {
            setValues({ ...values, name: event.currentTarget.value })
            setProblem(null)
          }}
        />
        <Textarea
          autosize
          minRows={2}
          maxRows={5}
          label={t(($) => $.form.description)}
          value={values.description}
          onChange={(event) => setValues({ ...values, description: event.currentTarget.value })}
        />
        <Stack gap={4}>
          <Text size="sm" fw={500}>
            {t(($) => $.form.mode)}
          </Text>
          <SegmentedControl
            fullWidth
            value={values.mode}
            data={MODES.map((mode) => ({ value: mode, label: t(($) => $.mode[mode].label) }))}
            onChange={(mode) =>
              setValues({ ...values, mode: mode === 'exclusive' ? 'exclusive' : 'additive' })
            }
          />
          <Text size="xs" c="var(--qc-text-2)">
            {t(($) => $.mode[values.mode].hint)}
          </Text>
        </Stack>
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose}>
            {t(($) => $.form.cancel)}
          </Button>
          <Button type="submit" loading={save.isPending} disabled={lengthProblem}>
            {submitLabel}
          </Button>
        </Group>
      </Stack>
    </form>
  )
}
