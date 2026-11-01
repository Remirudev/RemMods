import { ActionIcon, Button, Group, Tooltip } from '@mantine/core'
import { IconFolderOpen, IconRefresh } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { useRevealPaks } from '../../shared/game/queries'
import { LaunchButton } from './LaunchButton'
import { RunningBadge } from './RunningBadge'
import type { GameControlsProps } from './types'

export function GameControls({ running, tracking, refreshing, onRefresh }: GameControlsProps) {
  const { t } = useTranslation('mods')
  const reveal = useRevealPaks()

  return (
    <Group gap="sm" align="flex-start" wrap="nowrap">
      <RunningBadge running={running} />
      <LaunchButton running={running} tracking={tracking} />
      <Tooltip label={`${t(($) => $.header.refresh)} (F5)`}>
        <ActionIcon
          variant="default"
          size={36}
          aria-label={t(($) => $.header.refresh)}
          loading={refreshing}
          onClick={onRefresh}
        >
          <IconRefresh size={20} stroke={1.8} />
        </ActionIcon>
      </Tooltip>
      <Button
        variant="default"
        leftSection={<IconFolderOpen size={18} stroke={1.8} />}
        loading={reveal.isPending}
        onClick={() => reveal.mutate()}
      >
        {t(($) => $.header.openPaks)}
      </Button>
    </Group>
  )
}
