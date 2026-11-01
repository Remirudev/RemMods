import { Button, Group, Stack, TextInput } from '@mantine/core'
import { type SubmitEvent, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isIpcError } from '../../lib/ipc/ipc-error'
import { useSetGameLocation } from '../game/queries'
import { InvalidFolderAlert } from './InvalidFolderAlert'
import type { ManualPathFormProps } from './types'

export function ManualPathForm({ onSubmitted }: ManualPathFormProps) {
  const { t } = useTranslation('onboarding')
  const setLocation = useSetGameLocation()
  const [path, setPath] = useState('')
  const error = setLocation.error
  const appError = isIpcError(error) ? error.appError : null
  const notAbsolute = appError?.kind === 'invalidInput' && appError.field === 'path'

  const submit = (event: SubmitEvent<HTMLFormElement>) => {
    event.preventDefault()
    const trimmed = path.trim()
    if (trimmed === '') return
    setLocation.mutate({ kind: 'path', path: trimmed }, { onSuccess: () => onSubmitted?.() })
  }

  return (
    <form onSubmit={submit}>
      <Stack gap="sm">
        <Group align="flex-end" wrap="nowrap" gap="sm">
          <TextInput
            flex={1}
            label={t(($) => $.manual.label)}
            placeholder={t(($) => $.manual.placeholder)}
            value={path}
            onChange={(event) => setPath(event.currentTarget.value)}
            error={notAbsolute ? t(($) => $.manual.notAbsolute) : undefined}
            spellCheck={false}
            autoComplete="off"
          />
          <Button type="submit" loading={setLocation.isPending} disabled={path.trim() === ''}>
            {t(($) => $.manual.submit)}
          </Button>
        </Group>
        {appError?.kind === 'invalidGameFolder' && (
          <InvalidFolderAlert
            reason={appError.reason}
            path={setLocation.variables?.kind === 'path' ? setLocation.variables.path : path}
          />
        )}
      </Stack>
    </form>
  )
}
