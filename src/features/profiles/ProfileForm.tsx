import { Button, Group, Stack, Switch, Text, TextInput } from '@mantine/core'
import { type FormEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isIpcError } from '../../lib/ipc/ipc-error'
import { showErrorToast } from '../../lib/query/error-toast'
import type { ProfileFormProps } from './types'
import { useSaveProfile } from './use-profile-mutations'

const NAME_MAX_CHARS = 100

type FormProblem = 'taken' | 'invalid' | null

function problemOf(error: unknown): FormProblem {
  if (!isIpcError(error)) return null
  if (error.appError.kind === 'profileNameTaken') return 'taken'
  if (error.appError.kind === 'invalidInput') return 'invalid'
  return null
}

// "Save current state as a group": a name and whether the group turns every other mod off.
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
        <Text size="sm" c="var(--qc-text-2)">
          {t(($) => $.form.saveHint)}
        </Text>
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
        <Switch
          label={t(($) => $.form.exclusive)}
          description={t(($) => $.form.exclusiveHint)}
          checked={values.mode === 'exclusive'}
          onChange={(event) =>
            setValues({ ...values, mode: event.currentTarget.checked ? 'exclusive' : 'additive' })
          }
        />
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
