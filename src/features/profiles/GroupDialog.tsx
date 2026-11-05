import { Alert, Button, Group, Stack, Switch, TextInput } from '@mantine/core'
import { type FormEvent, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isIpcError } from '../../lib/ipc/ipc-error'
import { showErrorToast } from '../../lib/query/error-toast'
import { useModalLock } from '../../shared/use-modal-lock'
import { ClashAlert } from './ClashAlert'
import { pickEntries, pickItems, picksFromProfile, picksFromRows, withPicks } from './group-picks'
import { groupClashes } from './group-state'
import { ModPicker } from './ModPicker'
import type { GroupDialogProps, ModPick, Picks } from './types'
import { useSaveProfile } from './use-profile-mutations'

const NAME_MAX_CHARS = 100

type Problem = 'taken' | 'invalid' | 'mods' | null

function problemOf(error: unknown): Problem {
  if (!isIpcError(error)) return null
  const { appError } = error
  if (appError.kind === 'profileNameTaken') return 'taken'
  if (appError.kind === 'invalidInput') return appError.field === 'profileMods' ? 'mods' : 'invalid'
  return null
}

// Create or edit a group: its name, which mods it switches on and off, and whether switching it
// on also turns every other mod off.
export function GroupDialog({ profile, rows, onClose, modalId }: GroupDialogProps) {
  const { t, i18n } = useTranslation('profiles')
  const [name, setName] = useState(profile?.name ?? '')
  const [exclusive, setExclusive] = useState(profile?.mode === 'exclusive')
  const [picks, setPicks] = useState<Picks>(() => picksFromProfile(profile))
  const [problem, setProblem] = useState<Problem>(null)
  const save = useSaveProfile()
  useModalLock(modalId, save.isPending)

  const listed = useMemo(() => (profile?.mods ?? []).map((entry) => entry.key), [profile])
  const items = useMemo(() => pickItems(rows, listed, i18n.language), [rows, listed, i18n.language])
  const onKeys = useMemo(
    () => [...picks].filter(([, pick]) => pick === 'on').map(([key]) => key),
    [picks],
  )
  const clashes = useMemo(() => groupClashes(rows, onKeys), [rows, onKeys])

  const trimmed = name.trim()
  const lengthProblem = trimmed === '' || [...trimmed].length > NAME_MAX_CHARS
  const nameProblem =
    problem === 'taken' || problem === 'invalid'
      ? problem
      : lengthProblem && name !== ''
        ? 'invalid'
        : null

  const pick = (keys: readonly string[], value: ModPick) =>
    setPicks((now) => withPicks(now, keys, value))

  const submit = (event: FormEvent) => {
    event.preventDefault()
    if (lengthProblem || save.isPending) return
    save.mutate(
      {
        kind: 'picked',
        id: profile?.id ?? null,
        name: trimmed,
        description: profile?.description ?? '',
        mode: exclusive ? 'exclusive' : 'additive',
        mods: pickEntries(picks),
      },
      {
        onSuccess: onClose,
        onError: (error) => {
          const known = problemOf(error)
          if (known) setProblem(known)
          else showErrorToast(error)
        },
      },
    )
  }

  return (
    <form onSubmit={submit}>
      <Stack gap="md" data-testid="group-dialog">
        <TextInput
          data-autofocus
          label={t(($) => $.group.name)}
          value={name}
          error={nameProblem ? t(($) => $.form.problem[nameProblem]) : undefined}
          onChange={(event) => {
            setName(event.currentTarget.value)
            setProblem(null)
          }}
        />
        <ModPicker
          items={items}
          picks={picks}
          onPick={pick}
          onFromCurrent={() => setPicks(picksFromRows(rows))}
        />
        <ClashAlert clashes={clashes} />
        <Switch
          label={t(($) => $.form.exclusive)}
          description={t(($) => $.form.exclusiveHint)}
          checked={exclusive}
          onChange={(event) => setExclusive(event.currentTarget.checked)}
        />
        {problem === 'mods' && (
          <Alert color="red" role="alert">
            {t(($) => $.group.modsProblem)}
          </Alert>
        )}
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose} disabled={save.isPending}>
            {t(($) => $.form.cancel)}
          </Button>
          <Button type="submit" loading={save.isPending} disabled={lengthProblem}>
            {profile === null ? t(($) => $.group.create) : t(($) => $.group.save)}
          </Button>
        </Group>
      </Stack>
    </form>
  )
}
