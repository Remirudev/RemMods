import { Button, Stack, Text, Tooltip } from '@mantine/core'
import { IconPlayerPlay } from '@tabler/icons-react'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useLaunchGame } from '../../shared/game/queries'
import type { LaunchButtonProps } from './types'

const STARTING_TIMEOUT_MS = 60_000

export function LaunchButton({ running, tracking }: LaunchButtonProps) {
  const { t } = useTranslation('game')
  const launch = useLaunchGame()
  const [starting, setStarting] = useState(false)
  const otherInstall = tracking === 'noManifest'

  useEffect(() => {
    if (running === 'running') setStarting(false)
  }, [running])

  useEffect(() => {
    if (!starting) return
    const timer = window.setTimeout(() => setStarting(false), STARTING_TIMEOUT_MS)
    return () => window.clearTimeout(timer)
  }, [starting])

  const label =
    running === 'running'
      ? t(($) => $.launch.running)
      : starting
        ? t(($) => $.launch.starting)
        : t(($) => $.launch.button)

  return (
    <Stack gap={2} align="flex-end">
      <Tooltip label={otherInstall ? t(($) => $.launch.otherInstall) : t(($) => $.launch.hint)}>
        <div>
          <Button
            leftSection={<IconPlayerPlay size={18} stroke={1.8} />}
            disabled={running === 'running'}
            loading={starting && running !== 'running'}
            onClick={() => launch.mutate(undefined, { onSuccess: () => setStarting(true) })}
          >
            {label}
          </Button>
        </div>
      </Tooltip>
      {otherInstall && (
        <Text size="xs" c="dimmed" maw={260} ta="right">
          {t(($) => $.launch.otherInstall)}
        </Text>
      )}
    </Stack>
  )
}
