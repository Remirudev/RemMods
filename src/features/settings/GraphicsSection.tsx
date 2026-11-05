import { Alert, Button, Group, Paper, SegmentedControl, Stack, Text } from '@mantine/core'
import { IconAlertTriangle, IconInfoCircle } from '@tabler/icons-react'
import { useId } from 'react'
import { useTranslation } from 'react-i18next'
import type { GraphicsMode } from '../../lib/ipc/types'
import { GraphicsStepBanner } from './GraphicsStepBanner'
import {
  canChangeMode,
  GRAPHICS_MODES,
  graphicsSupported,
  isGraphicsMode,
  offersSoftware,
} from './graphics-model'
import { useGraphics, usePlatformInfo, useSetGraphicsMode } from './queries'
import { SectionHeading } from './SectionHeading'
import type { SegmentOption } from './types'

// Linux only: Windows reports no graphics fallback, so the section is not rendered there.
export function GraphicsSection() {
  const { t } = useTranslation('settings')
  const labelId = useId()
  const supported = graphicsSupported(usePlatformInfo().data)
  const graphics = useGraphics(supported)
  const setMode = useSetGraphicsMode()
  const state = graphics.data

  if (!supported || (state && !state.supported)) return null

  const options: SegmentOption<GraphicsMode>[] = GRAPHICS_MODES.map((mode) => ({
    value: mode,
    label: t(($) => $.graphics.modes[mode]),
  }))
  const changeMode = (value: string) => {
    if (isGraphicsMode(value) && value !== state?.mode) setMode.mutate(value)
  }

  return (
    <Paper withBorder p="lg" data-testid="graphics-section">
      <Stack gap="lg">
        <SectionHeading
          title={t(($) => $.graphics.title)}
          description={t(($) => $.graphics.description)}
        />
        <GraphicsStepBanner />
        <Stack gap="xs">
          <Text id={labelId} fw={600} size="sm">
            {t(($) => $.graphics.label)}
          </Text>
          <SegmentedControl
            aria-labelledby={labelId}
            data={options}
            value={state?.mode ?? 'auto'}
            disabled={!canChangeMode(state, setMode.isPending)}
            onChange={changeMode}
          />
          <Text size="xs" c="dimmed">
            {t(($) => $.graphics.hint)}
          </Text>
        </Stack>
        {state?.readOnly && (
          <Alert color="yellow" icon={<IconAlertTriangle size={20} stroke={1.8} />}>
            {t(($) => $.graphics.readOnly)}
          </Alert>
        )}
        {state?.safeFlag && (
          <Alert color="blue" icon={<IconInfoCircle size={20} stroke={1.8} />}>
            <Stack gap="sm">
              <Text size="sm">{t(($) => $.graphics.safeFlag)}</Text>
              {offersSoftware(state) && (
                <Group>
                  <Button
                    size="xs"
                    variant="default"
                    disabled={setMode.isPending}
                    onClick={() => setMode.mutate('software')}
                  >
                    {t(($) => $.graphics.alwaysSoftware)}
                  </Button>
                </Group>
              )}
            </Stack>
          </Alert>
        )}
        {state?.userEnv && (
          <Alert color="gray" icon={<IconInfoCircle size={20} stroke={1.8} />}>
            {t(($) => $.graphics.userEnv)}
          </Alert>
        )}
      </Stack>
    </Paper>
  )
}
