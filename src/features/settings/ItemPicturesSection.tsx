import { Button, Group, Paper, Stack, Switch, Text } from '@mantine/core'
import { IconRefresh } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { formatSize } from '../../shared/format-size'
import { useAssetsRefresh, useAssetsStatus, useSettings, useUpdateSettings } from './queries'
import { SectionHeading } from './SectionHeading'
import { appearanceUpdate } from './settings-update'

export function ItemPicturesSection() {
  const { t, i18n } = useTranslation('settings')
  const status = useAssetsStatus(i18n.language).data
  const refresh = useAssetsRefresh(i18n.language)
  const settings = useSettings().data
  const updateSettings = useUpdateSettings()
  const numbers = new Intl.NumberFormat(i18n.language)

  return (
    <Paper withBorder p="lg">
      <Stack gap="lg">
        <SectionHeading
          title={t(($) => $.itemPictures.title)}
          description={t(($) => $.itemPictures.description)}
        />
        {status && status.pictures + status.names > 0 && (
          <Text size="sm">
            {t(($) => $.itemPictures.summary, {
              pictures: numbers.format(status.pictures),
              size: formatSize(status.bytes, i18n.language) ?? '',
              names: numbers.format(status.names),
            })}
          </Text>
        )}
        {status && status.pictures + status.names === 0 && (
          <Text size="sm" c="var(--qc-text-2)">
            {t(($) => $.itemPictures.empty)}
          </Text>
        )}
        {status && status.failed > 0 && (
          <Text size="sm" c="var(--qc-text-2)">
            {t(($) => $.itemPictures.failed, { count: status.failed })}
          </Text>
        )}
        <Switch
          label={t(($) => $.itemPictures.hide)}
          description={t(($) => $.itemPictures.hideHint)}
          checked={settings?.appearance.hidePictures ?? false}
          disabled={!settings || settings.readOnly}
          onChange={(event) =>
            updateSettings.mutate(appearanceUpdate({ hidePictures: event.currentTarget.checked }))
          }
        />
        <Switch
          label={t(($) => $.itemPictures.hoverPreview)}
          description={t(($) => $.itemPictures.hoverPreviewHint)}
          checked={settings?.appearance.hoverPreview ?? true}
          disabled={!settings || settings.readOnly}
          onChange={(event) =>
            updateSettings.mutate(appearanceUpdate({ hoverPreview: event.currentTarget.checked }))
          }
        />
        <Group>
          <Button
            variant="default"
            leftSection={<IconRefresh size={18} stroke={1.8} />}
            loading={refresh.isPending}
            onClick={() => refresh.mutate()}
          >
            {t(($) => $.itemPictures.refresh)}
          </Button>
        </Group>
      </Stack>
    </Paper>
  )
}
