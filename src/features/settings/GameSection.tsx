import { Accordion, Button, DataList, Group, Paper, Stack, Text } from '@mantine/core'
import { IconFolderOpen, IconFolderSearch, IconRefresh } from '@tabler/icons-react'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { DetectionNotes } from '../../shared/components/DetectionNotes'
import { GameLocationChooser } from '../../shared/components/GameLocationChooser'
import { useDetectGame, useGameStatus, useRevealPaks } from '../../shared/game/queries'
import { useFolderChooser } from '../../shared/game/use-folder-chooser'
import { SectionHeading } from './SectionHeading'
import type { GameRow } from './types'

const OTHER_INSTALLATIONS = 'other'

export function GameSection() {
  const { t } = useTranslation('settings')
  const { t: tGame } = useTranslation('game')
  const status = useGameStatus().data
  const detect = useDetectGame()
  const reveal = useRevealPaks()
  const chooser = useFolderChooser()
  const [opened, setOpened] = useState<string | null>(null)
  const cancelled = chooser.cancelled

  useEffect(() => {
    if (cancelled) setOpened(OTHER_INSTALLATIONS)
  }, [cancelled])

  if (!status) return null
  const location = status.location
  const install = location.kind === 'ready' ? location.install : null
  const steamKind = install?.steamKind ?? null
  const rows: GameRow[] = install
    ? [
        { label: t(($) => $.game.labels.folder), value: install.root },
        { label: t(($) => $.game.labels.paks), value: install.paks },
        { label: t(($) => $.game.labels.source), value: tGame(($) => $.sources[install.source]) },
        {
          label: t(($) => $.game.labels.build),
          value: tGame(($) => $.tracking[install.updateTracking], {
            build: install.buildId ?? '',
          }),
        },
        ...(steamKind
          ? [{ label: t(($) => $.game.labels.steam), value: tGame(($) => $.steamKinds[steamKind]) }]
          : []),
      ]
    : []
  const showChooser = status.candidates.length > 0 || cancelled

  return (
    <Paper withBorder p="lg">
      <Stack gap="lg">
        <SectionHeading title={t(($) => $.game.title)} description={t(($) => $.game.description)} />
        {install ? (
          <DataList labelWidth={180}>
            {rows.map((row) => (
              <DataList.Item key={row.label}>
                <DataList.ItemLabel>{row.label}</DataList.ItemLabel>
                <DataList.ItemValue ff="monospace" style={{ overflowWrap: 'anywhere' }}>
                  <bdi>{row.value}</bdi>
                </DataList.ItemValue>
              </DataList.Item>
            ))}
          </DataList>
        ) : (
          <Text size="sm">
            {location.kind === 'missing'
              ? t(($) => $.game.missing, { path: location.path })
              : t(($) => $.game.notSet)}
          </Text>
        )}
        <Group>
          <Button
            variant="default"
            leftSection={<IconFolderSearch size={18} stroke={1.8} />}
            loading={chooser.picking}
            onClick={chooser.pick}
          >
            {t(($) => $.game.change)}
          </Button>
          <Button
            variant="default"
            leftSection={<IconRefresh size={18} stroke={1.8} />}
            loading={detect.isPending}
            onClick={() => detect.mutate()}
          >
            {t(($) => $.game.detect)}
          </Button>
          {install && (
            <Button
              variant="default"
              leftSection={<IconFolderOpen size={18} stroke={1.8} />}
              loading={reveal.isPending}
              onClick={() => reveal.mutate()}
            >
              {t(($) => $.game.reveal)}
            </Button>
          )}
        </Group>
        {showChooser && (
          <Accordion variant="contained" value={opened} onChange={setOpened}>
            <Accordion.Item value={OTHER_INSTALLATIONS}>
              <Accordion.Control>{t(($) => $.game.other)}</Accordion.Control>
              <Accordion.Panel>
                <GameLocationChooser status={status} chooser={chooser} withActions={false} />
              </Accordion.Panel>
            </Accordion.Item>
          </Accordion>
        )}
        {status.notes.length > 0 && <DetectionNotes notes={status.notes} />}
      </Stack>
    </Paper>
  )
}
