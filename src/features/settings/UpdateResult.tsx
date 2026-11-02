import { Button, Group, ScrollArea, Stack, Text } from '@mantine/core'
import { IconDownload } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import classes from './settings.module.css'
import type { UpdateResultProps } from './types'
import { formatPublished, updateOffer } from './update-model'

export function UpdateResult({ check, channel, installing, onInstall }: UpdateResultProps) {
  const { t, i18n } = useTranslation('settings')

  if (check.kind === 'upToDate') {
    return (
      <Text size="sm" data-testid="update-result">
        {t(($) => $.updates.upToDate, { version: channel.current })}
      </Text>
    )
  }

  const offer = updateOffer(channel)
  const published = formatPublished(check.date, i18n.language)

  return (
    <Stack gap="sm" data-testid="update-result">
      <Stack gap={2}>
        <Text size="sm" fw={600}>
          {t(($) => $.updates.available, { version: check.version })}
        </Text>
        {published && (
          <Text size="xs" c="var(--qc-text-2)">
            {t(($) => $.updates.published, { date: published })}
          </Text>
        )}
      </Stack>
      {check.notes && (
        <Stack gap={4}>
          <Text size="xs" fw={600}>
            {t(($) => $.updates.notes)}
          </Text>
          <ScrollArea.Autosize mah={240} type="auto">
            <Text component="pre" size="sm" className={classes.notes}>
              {check.notes}
            </Text>
          </ScrollArea.Autosize>
        </Stack>
      )}
      {offer.kind === 'install' && (
        <Group>
          <Button
            leftSection={<IconDownload size={18} stroke={1.8} />}
            loading={installing}
            onClick={() => onInstall(check.version)}
          >
            {t(($) => $.updates.install)}
          </Button>
        </Group>
      )}
    </Stack>
  )
}
