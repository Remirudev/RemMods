import { Alert, Button, Group, Stack, Text } from '@mantine/core'
import { IconAlertTriangle } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { graphicsSupported } from './graphics-model'
import { useAckGraphicsStep, useGraphics, usePlatformInfo } from './queries'

// After a crashed start RemMods stepped down one graphics mode; the user keeps it or undoes it.
// Rendered by the app shell and by the Graphics section; never on Windows.
export function GraphicsStepBanner() {
  const { t } = useTranslation('settings')
  const supported = graphicsSupported(usePlatformInfo().data)
  const graphics = useGraphics(supported)
  const ack = useAckGraphicsStep()
  const step = graphics.data?.supported ? graphics.data.step : null

  if (!supported || !step) return null

  return (
    <Alert
      color="yellow"
      title={t(($) => $.graphics.step.title)}
      icon={<IconAlertTriangle size={20} stroke={1.8} />}
      data-testid="graphics-step-banner"
    >
      <Stack gap="sm">
        <Text size="sm">
          {t(($) => $.graphics.step.body, {
            from: t(($) => $.graphics.modes[step.from]),
            to: t(($) => $.graphics.modes[step.to]),
          })}
        </Text>
        <Group gap="sm">
          <Button size="xs" disabled={ack.isPending} onClick={() => ack.mutate(false)}>
            {t(($) => $.graphics.step.keep)}
          </Button>
          <Button
            size="xs"
            variant="default"
            disabled={ack.isPending}
            onClick={() => ack.mutate(true)}
          >
            {t(($) => $.graphics.step.revert)}
          </Button>
        </Group>
      </Stack>
    </Alert>
  )
}
