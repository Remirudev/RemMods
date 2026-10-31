import { Group, Paper, SegmentedControl, Slider, Stack, Text } from '@mantine/core'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { ColorScheme, TitleBarMode } from '../../lib/ipc/types'
import { UI_SCALE_PERCENT } from '../../shared/ui-scale'
import { useSettings, useSetUiScale, useUpdateSettings } from './queries'
import { SectionHeading } from './SectionHeading'
import { appearanceUpdate, DEFAULT_APPEARANCE } from './settings-update'
import type { SegmentOption } from './types'

export function AppearanceSection() {
  const { t, i18n } = useTranslation('settings')
  const schemeLabelId = useId()
  const titleBarLabelId = useId()
  const settings = useSettings()
  const updateSettings = useUpdateSettings()
  const setUiScale = useSetUiScale()
  const [draftPercent, setDraftPercent] = useState<number | null>(null)

  const appearance = settings.data?.appearance ?? DEFAULT_APPEARANCE
  const disabled = !settings.data || settings.data.readOnly
  const percent = draftPercent ?? Math.round(appearance.uiScale * 100)
  const percentFormat = new Intl.NumberFormat(i18n.language, { style: 'percent' })
  const formatPercent = (value: number) => percentFormat.format(value / 100)

  const schemeOptions: SegmentOption<ColorScheme>[] = [
    { value: 'dark', label: t(($) => $.appearance.scheme.dark) },
    { value: 'light', label: t(($) => $.appearance.scheme.light) },
    { value: 'system', label: t(($) => $.appearance.scheme.system) },
  ]
  const titleBarOptions: SegmentOption<TitleBarMode>[] = [
    { value: 'custom', label: t(($) => $.appearance.titleBar.custom) },
    { value: 'system', label: t(($) => $.appearance.titleBar.system) },
  ]

  const commitPercent = (value: number) => {
    setUiScale.mutate(value / 100, { onSettled: () => setDraftPercent(null) })
  }

  return (
    <Paper withBorder p="lg">
      <Stack gap="lg">
        <SectionHeading
          title={t(($) => $.appearance.title)}
          description={t(($) => $.appearance.description)}
        />
        <Stack gap="xs">
          <Text id={schemeLabelId} fw={600} size="sm">
            {t(($) => $.appearance.scheme.label)}
          </Text>
          <SegmentedControl
            aria-labelledby={schemeLabelId}
            data={schemeOptions}
            value={appearance.scheme}
            disabled={disabled}
            onChange={(scheme) => updateSettings.mutate(appearanceUpdate({ scheme }))}
          />
        </Stack>
        <Stack gap="xs">
          <Group justify="space-between">
            <Text fw={600} size="sm">
              {t(($) => $.appearance.uiScale.label)}
            </Text>
            <Text size="sm" c="dimmed">
              {formatPercent(percent)}
            </Text>
          </Group>
          <Slider
            min={UI_SCALE_PERCENT.min}
            max={UI_SCALE_PERCENT.max}
            step={UI_SCALE_PERCENT.step}
            value={percent}
            disabled={disabled}
            thumbLabel={t(($) => $.appearance.uiScale.label)}
            label={formatPercent}
            onChange={setDraftPercent}
            onChangeEnd={commitPercent}
          />
          <Text size="xs" c="dimmed">
            {t(($) => $.appearance.uiScale.hint)}
          </Text>
        </Stack>
        <Stack gap="xs">
          <Text id={titleBarLabelId} fw={600} size="sm">
            {t(($) => $.appearance.titleBar.label)}
          </Text>
          <SegmentedControl
            aria-labelledby={titleBarLabelId}
            data={titleBarOptions}
            value={appearance.titleBar}
            disabled={disabled}
            onChange={(titleBar) => updateSettings.mutate(appearanceUpdate({ titleBar }))}
          />
          <Text size="xs" c="dimmed">
            {t(($) => $.appearance.titleBar.hint)}
          </Text>
        </Stack>
      </Stack>
    </Paper>
  )
}
