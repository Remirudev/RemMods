import { Group } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { PanelHeader } from '../../shared/components/PanelHeader'
import { ConflictsChip } from './ConflictsChip'
import { GameControls } from './GameControls'
import { QuickApplyMenu } from './QuickApplyMenu'
import type { ModsHeaderProps } from './types'

export function ModsHeader({
  snapshot,
  running,
  tracking,
  refreshing,
  onRefresh,
}: ModsHeaderProps) {
  const { t, i18n } = useTranslation('mods')
  const { t: tShell } = useTranslation('shell')
  const scanned = snapshot ? new Date(snapshot.scannedAt) : null
  const time =
    scanned && !Number.isNaN(scanned.getTime()) ? scanned.toLocaleTimeString(i18n.language) : null

  return (
    <PanelHeader
      title={tShell(($) => $.panels.mods.title)}
      description={time ? t(($) => $.header.scannedAt, { time }) : undefined}
      actions={
        <Group gap="sm" align="flex-start" wrap="nowrap">
          {snapshot && <ConflictsChip rows={snapshot.rows} />}
          {snapshot && <QuickApplyMenu />}
          <GameControls
            running={running}
            tracking={tracking}
            refreshing={refreshing}
            onRefresh={onRefresh}
          />
        </Group>
      }
    />
  )
}
