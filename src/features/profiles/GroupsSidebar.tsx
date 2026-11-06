import { ActionIcon, Button, Group, ScrollArea, Stack, Text, Tooltip } from '@mantine/core'
import {
  IconLayoutSidebarLeftCollapse,
  IconLayoutSidebarLeftExpand,
  IconPlus,
} from '@tabler/icons-react'
import { type MouseEvent, useEffect, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import type { ApplyDirection } from '../../lib/ipc/types'
import { useGroupFilterStore } from '../../shared/group-filter-store'
import { CollapsedGroups } from './CollapsedGroups'
import { GroupBulkBar } from './GroupBulkBar'
import { GroupRow } from './GroupRow'
import { GroupsMenu } from './GroupsMenu'
import { useGroupPrefs } from './group-prefs-store'
import { bulkTargets, groupItems, picksAfterClick, switchTargets } from './group-state'
import classes from './groups.module.css'
import { openGroupDialog } from './profile-modals'
import { useProfiles } from './queries'
import type { GroupItem, GroupsSidebarProps } from './types'
import { useGroupSwitch } from './use-group-switch'

function profilesOf(items: readonly GroupItem[]) {
  return items.map((item) => item.summary.profile)
}

// Groups beside the mod list: one switch per group, the name narrows the list to its mods. Ctrl and
// Shift pick several, like a file list, and a switch then moves all of them.
export function GroupsSidebar({ rows, footer }: GroupsSidebarProps) {
  const { t, i18n } = useTranslation('profiles')
  const profiles = useProfiles()
  const view = profiles.data ?? undefined
  const readOnly = view?.readOnly ?? true
  const collapsed = useGroupPrefs((state) => state.collapsed)
  const setCollapsed = useGroupPrefs((state) => state.setCollapsed)
  const picked = useGroupFilterStore((state) => state.picked)
  const { setGroups, pickGroups, clearPicks } = useGroupFilterStore.getState()
  const { switchGroups, busyIds } = useGroupSwitch()
  const unnamed = t(($) => $.unnamed)
  const items = useMemo(
    () => groupItems(view, rows, i18n.language, unnamed),
    [view, rows, i18n.language, unnamed],
  )

  // The list filter follows edits of its groups, and a deleted group leaves it.
  useEffect(() => {
    if (!view) return
    setGroups(
      items.map((item) => ({
        id: item.summary.profile.id,
        name: item.title,
        keys: item.onKeys,
      })),
    )
  }, [items, view, setGroups])

  const direction = (item: GroupItem): ApplyDirection => (item.lit ? 'off' : 'on')
  const onSwitchOne = (item: GroupItem) => switchGroups(profilesOf([item]), direction(item))
  const onSwitch = (item: GroupItem) =>
    switchGroups(profilesOf(switchTargets(items, picked, item)), direction(item))
  const onSwitchAll = (wanted: ApplyDirection) =>
    switchGroups(profilesOf(bulkTargets(items, picked)), wanted)
  const onShow = (item: GroupItem, event: MouseEvent) => {
    const { picked: current, anchor } = useGroupFilterStore.getState()
    const ids = profilesOf(items).map((profile) => profile.id)
    const next = picksAfterClick(current, anchor, ids, item.summary.profile.id, {
      shift: event.shiftKey,
      ctrl: event.ctrlKey || event.metaKey,
    })
    pickGroups(next.selection, next.anchor)
  }

  if (collapsed) {
    return (
      <nav
        className={`${classes.sidebar} ${classes.collapsed}`}
        aria-label={t(($) => $.sidebar.title)}
        data-testid="groups-sidebar"
      >
        <Tooltip label={t(($) => $.sidebar.expand)} position="right" withinPortal>
          <ActionIcon
            variant="subtle"
            color="gray"
            aria-label={t(($) => $.sidebar.expand)}
            onClick={() => setCollapsed(false)}
          >
            <IconLayoutSidebarLeftExpand size={18} stroke={1.8} />
          </ActionIcon>
        </Tooltip>
        <CollapsedGroups
          items={items}
          busyIds={busyIds}
          readOnly={readOnly}
          onSwitch={onSwitchOne}
        />
      </nav>
    )
  }

  return (
    <nav
      className={classes.sidebar}
      aria-label={t(($) => $.sidebar.title)}
      data-testid="groups-sidebar"
    >
      <Group justify="space-between" wrap="nowrap" gap={4} mb="xs">
        <Text fw={600} size="sm">
          {t(($) => $.sidebar.title)}
        </Text>
        <Group gap={2} wrap="nowrap">
          <Tooltip label={t(($) => $.sidebar.newGroup)} withinPortal>
            <ActionIcon
              variant="subtle"
              color="gray"
              aria-label={t(($) => $.sidebar.newGroup)}
              disabled={readOnly}
              onClick={() => openGroupDialog(null, rows)}
            >
              <IconPlus size={18} stroke={1.8} />
            </ActionIcon>
          </Tooltip>
          <GroupsMenu readOnly={readOnly} />
          <Tooltip label={t(($) => $.sidebar.collapse)} withinPortal>
            <ActionIcon
              variant="subtle"
              color="gray"
              aria-label={t(($) => $.sidebar.collapse)}
              onClick={() => setCollapsed(true)}
            >
              <IconLayoutSidebarLeftCollapse size={18} stroke={1.8} />
            </ActionIcon>
          </Tooltip>
        </Group>
      </Group>
      {view?.readOnly && (
        <Text size="xs" c="yellow" role="status" mb="xs">
          {t(($) => $.sidebar.readOnly)}
        </Text>
      )}
      {view?.problem && (
        <Text size="xs" c="red" role="alert" mb="xs">
          {t(($) => $.sidebar.problem[view.problem ?? 'unreadable'])}
        </Text>
      )}
      {profiles.isError && (
        <Text size="xs" c="red" role="alert" mb="xs">
          {t(($) => $.sidebar.loadError)}
        </Text>
      )}
      {items.length > 1 && (
        <GroupBulkBar
          selected={picked.length}
          disabled={readOnly || busyIds.length > 0}
          onSwitch={onSwitchAll}
          onClear={clearPicks}
        />
      )}
      <ScrollArea className={classes.list} type="hover">
        {view && items.length === 0 ? (
          <Stack gap="sm" align="flex-start">
            <Text size="sm" c="var(--qc-text-2)">
              {t(($) => $.sidebar.empty)}
            </Text>
            <Button
              size="compact-sm"
              variant="light"
              leftSection={<IconPlus size={16} stroke={1.8} />}
              disabled={readOnly}
              onClick={() => openGroupDialog(null, rows)}
            >
              {t(($) => $.sidebar.newGroup)}
            </Button>
          </Stack>
        ) : (
          <Stack gap={2}>
            {items.map((item) => (
              <GroupRow
                key={item.summary.profile.id}
                item={item}
                rows={rows}
                active={picked.includes(item.summary.profile.id)}
                busy={busyIds.includes(item.summary.profile.id)}
                readOnly={readOnly}
                onSwitch={onSwitch}
                onShow={onShow}
              />
            ))}
          </Stack>
        )}
      </ScrollArea>
      {footer}
    </nav>
  )
}
