import { Alert, Button, Collapse, Group, Stack, Text, Title } from '@mantine/core'
import { IconFolderSearch, IconRefresh } from '@tabler/icons-react'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { describeError } from '../../lib/i18n/error-messages'
import { isIpcError } from '../../lib/ipc/ipc-error'
import { useDetectGame, useSetGameLocation } from '../game/queries'
import { GameCandidateList } from './GameCandidateList'
import { InvalidFolderAlert } from './InvalidFolderAlert'
import { ManualPathForm } from './ManualPathForm'
import type { GameLocationChooserProps } from './types'

export function GameLocationChooser({ status, chooser, withActions }: GameLocationChooserProps) {
  const { t } = useTranslation('onboarding')
  const setLocation = useSetGameLocation()
  const detect = useDetectGame()
  const [manualOpen, setManualOpen] = useState(false)
  const cancelled = chooser.cancelled

  useEffect(() => {
    if (cancelled) setManualOpen(true)
  }, [cancelled])

  const candidateError = isIpcError(setLocation.error) ? describeError(setLocation.error) : null
  const busyId =
    setLocation.isPending && setLocation.variables?.kind === 'candidate'
      ? setLocation.variables.id
      : null

  return (
    <Stack gap="lg">
      <Stack gap="sm">
        {withActions && (
          <Title order={3} fz="h5">
            {t(($) => $.found.title)}
          </Title>
        )}
        {status.candidates.length === 0 ? (
          <Text size="sm" c="var(--qc-text-2)">
            {t(($) => $.found.none)}
          </Text>
        ) : (
          <GameCandidateList
            candidates={status.candidates}
            busyId={busyId}
            onUse={(id) => setLocation.mutate({ kind: 'candidate', id })}
          />
        )}
        {candidateError && (
          <Alert color="red" title={candidateError.title} role="alert">
            {candidateError.message}
          </Alert>
        )}
      </Stack>
      {chooser.invalid && (
        <InvalidFolderAlert reason={chooser.invalid.reason} path={chooser.invalid.path} />
      )}
      {withActions && (
        <Group gap="sm">
          <Button
            leftSection={<IconFolderSearch size={18} stroke={1.8} />}
            loading={chooser.picking}
            onClick={chooser.pick}
          >
            {t(($) => $.choose.button)}
          </Button>
          <Button
            variant="default"
            leftSection={<IconRefresh size={18} stroke={1.8} />}
            loading={detect.isPending}
            onClick={() => detect.mutate()}
          >
            {t(($) => $.detectAgain)}
          </Button>
        </Group>
      )}
      <Stack gap="xs">
        {withActions && (
          <Text size="sm" c="var(--qc-text-2)">
            {t(($) => $.choose.hint)}
          </Text>
        )}
        {chooser.cancelled && (
          <Text size="sm" role="status">
            {t(($) => $.choose.cancelled)}
          </Text>
        )}
        <Button
          variant="subtle"
          size="compact-sm"
          w="fit-content"
          aria-expanded={manualOpen}
          onClick={() => setManualOpen((open) => !open)}
        >
          {t(($) => $.manual.toggle)}
        </Button>
        <Collapse expanded={manualOpen} keepMounted={false}>
          <ManualPathForm />
        </Collapse>
      </Stack>
    </Stack>
  )
}
