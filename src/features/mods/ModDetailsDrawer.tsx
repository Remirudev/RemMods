import { Button, Code, DataList, Drawer, Group, ScrollArea, Stack, Text } from '@mantine/core'
import { IconFolderOpen } from '@tabler/icons-react'
import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { formatSize } from '../../shared/format-size'
import { ConflictList } from './ConflictList'
import { ContentConflictList } from './ContentConflictList'
import { ContentSection } from './ContentSection'
import { DetailsFiles } from './DetailsFiles'
import { DrawerActions } from './DrawerActions'
import { DrawerSection } from './DrawerSection'
import { displayState } from './display-state'
import { IssueText } from './IssueText'
import { MetadataInfo } from './MetadataInfo'
import classes from './mods.module.css'
import { useModsUiStore } from './mods-ui-store'
import { useRevealEntry } from './queries'
import type { ModDetailsDrawerProps } from './types'

export function ModDetailsDrawer({ rows }: ModDetailsDrawerProps) {
  const { t, i18n } = useTranslation('mods')
  const detailsRef = useModsUiStore((state) => state.detailsRef)
  const closeDetails = useModsUiStore((state) => state.closeDetails)
  const reveal = useRevealEntry()
  const row =
    detailsRef === null ? undefined : rows.find((candidate) => candidate.modRef === detailsRef)

  // The store is read again here: a reconcile that followed a renamed row ran after this render.
  useEffect(() => {
    const current = useModsUiStore.getState().detailsRef
    if (current !== null && !rows.some((candidate) => candidate.modRef === current)) closeDetails()
  }, [rows, closeDetails])

  const state = row ? displayState(row) : null
  const companions = row ? row.payload.liveCompanion + row.payload.offCompanion : 0

  return (
    <Drawer
      opened={row !== undefined}
      onClose={closeDetails}
      position="right"
      size="lg"
      overlayProps={{ backgroundOpacity: 0.45, blur: 0 }}
      scrollAreaComponent={ScrollArea.Autosize}
      closeButtonProps={{ 'aria-label': t(($) => $.drawer.close) }}
      title={row ? <bdi>{row.name}</bdi> : null}
    >
      {row && state && (
        <Stack gap="xl" data-testid="mod-details">
          <Stack gap="xs">
            <DataList labelWidth={120}>
              <DataList.Item>
                <DataList.ItemLabel>{t(($) => $.drawer.key)}</DataList.ItemLabel>
                <DataList.ItemValue>
                  <Code className={classes.keyCode}>{row.key}</Code>
                </DataList.ItemValue>
              </DataList.Item>
              <DataList.Item>
                <DataList.ItemLabel>{t(($) => $.drawer.folder)}</DataList.ItemLabel>
                <DataList.ItemValue ff="monospace" style={{ overflowWrap: 'anywhere' }}>
                  <bdi>{row.location}</bdi>
                </DataList.ItemValue>
              </DataList.Item>
              <DataList.Item>
                <DataList.ItemLabel>{t(($) => $.drawer.kind)}</DataList.ItemLabel>
                <DataList.ItemValue>{t(($) => $.kinds[row.kind])}</DataList.ItemValue>
              </DataList.Item>
              <DataList.Item>
                <DataList.ItemLabel>{t(($) => $.drawer.state)}</DataList.ItemLabel>
                <DataList.ItemValue>
                  {t(($) => $.states[state])}
                  <Text size="xs" c="var(--qc-text-2)">
                    {t(($) => $.stateHints[state])}
                  </Text>
                </DataList.ItemValue>
              </DataList.Item>
              <DataList.Item>
                <DataList.ItemLabel>{t(($) => $.drawer.size)}</DataList.ItemLabel>
                <DataList.ItemValue>
                  {formatSize(row.sizeBytes, i18n.language) ?? t(($) => $.size.unknown)}
                </DataList.ItemValue>
              </DataList.Item>
              <DataList.Item>
                <DataList.ItemLabel>{t(($) => $.drawer.payload)}</DataList.ItemLabel>
                <DataList.ItemValue>
                  {t(($) => $.drawer.payloadValue, {
                    livePak: row.payload.livePak,
                    offPak: row.payload.offPak,
                  })}
                  {'; '}
                  {t(($) => $.drawer.payloadCompanions, { count: companions })}
                </DataList.ItemValue>
              </DataList.Item>
            </DataList>
          </Stack>

          <DrawerSection title={t(($) => $.drawer.info)}>
            <MetadataInfo key={row.modRef} row={row} />
          </DrawerSection>

          <DrawerSection title={t(($) => $.drawer.conflicts)}>
            <ConflictList conflicts={row.conflictsWith} />
          </DrawerSection>

          <DrawerSection title={t(($) => $.drawer.issues)}>
            {row.issues.length === 0 ? (
              <Text size="sm">{t(($) => $.drawer.issuesNone)}</Text>
            ) : (
              <Stack gap="xs">
                {row.issues.map((issue) => (
                  <IssueText key={JSON.stringify(issue)} issue={issue} root={row.root} />
                ))}
              </Stack>
            )}
          </DrawerSection>

          <DrawerSection title={t(($) => $.drawer.files)}>
            <DetailsFiles modRef={row.modRef} />
          </DrawerSection>

          {row.contentConflicts.length > 0 && (
            <DrawerSection title={t(($) => $.contentConflicts.title)}>
              <ContentConflictList conflicts={row.contentConflicts} />
            </DrawerSection>
          )}

          <DrawerSection title={t(($) => $.content.title)}>
            <ContentSection key={row.modRef} row={row} />
          </DrawerSection>

          <Group gap="sm">
            <Button
              variant="default"
              w="fit-content"
              leftSection={<IconFolderOpen size={18} stroke={1.8} />}
              loading={reveal.isPending}
              onClick={() => reveal.mutate(row.modRef)}
            >
              {t(($) => $.drawer.reveal)}
            </Button>
          </Group>
          <DrawerActions row={row} />
        </Stack>
      )}
    </Drawer>
  )
}
