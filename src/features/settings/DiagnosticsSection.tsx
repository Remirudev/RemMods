import { Button, DataList, Group, Paper, Stack } from '@mantine/core'
import { IconFolderOpen } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { graphicsSupported } from './graphics-model'
import { useGraphics, useOpenLogs, usePlatformInfo, useUpdateChannel } from './queries'
import { SectionHeading } from './SectionHeading'
import type { DiagnosticsRow } from './types'

export function DiagnosticsSection() {
  const { t } = useTranslation('settings')
  const platformInfo = usePlatformInfo()
  const openLogs = useOpenLogs()
  const info = platformInfo.data
  const install = useUpdateChannel().data?.install
  const linuxGraphics = graphicsSupported(info)
  const effective = useGraphics(linuxGraphics).data?.effective

  const rows: DiagnosticsRow[] = [
    {
      label: t(($) => $.diagnostics.labels.version),
      value: info?.appVersion,
    },
    {
      label: t(($) => $.diagnostics.labels.os),
      value: info && t(($) => $.diagnostics.osNames[info.os]),
    },
    {
      label: t(($) => $.diagnostics.labels.arch),
      value: info?.arch,
    },
    {
      label: t(($) => $.diagnostics.labels.webview),
      value: info?.webviewVersion,
    },
    {
      label: t(($) => $.diagnostics.labels.installedAs),
      value: install && t(($) => $.updates.kinds[install]),
    },
    ...(linuxGraphics
      ? [
          {
            label: t(($) => $.diagnostics.labels.graphicsMode),
            value: effective && t(($) => $.graphics.modes[effective]),
          },
        ]
      : []),
    {
      label: t(($) => $.diagnostics.labels.configDir),
      value: info?.paths.config,
    },
    {
      label: t(($) => $.diagnostics.labels.dataDir),
      value: info?.paths.data,
    },
    {
      label: t(($) => $.diagnostics.labels.logsDir),
      value: info?.paths.logs,
    },
  ]

  return (
    <Paper withBorder p="lg">
      <Stack gap="lg">
        <SectionHeading
          title={t(($) => $.diagnostics.title)}
          description={t(($) => $.diagnostics.description)}
        />
        <DataList labelWidth={180}>
          {rows.map((row) => (
            <DataList.Item key={row.label}>
              <DataList.ItemLabel>{row.label}</DataList.ItemLabel>
              <DataList.ItemValue ff="monospace">
                {row.value ?? t(($) => $.diagnostics.unavailable)}
              </DataList.ItemValue>
            </DataList.Item>
          ))}
        </DataList>
        <Group>
          <Button
            variant="default"
            leftSection={<IconFolderOpen size={18} stroke={1.8} />}
            loading={openLogs.isPending}
            onClick={() => openLogs.mutate()}
          >
            {t(($) => $.diagnostics.openLogs)}
          </Button>
        </Group>
      </Stack>
    </Paper>
  )
}
