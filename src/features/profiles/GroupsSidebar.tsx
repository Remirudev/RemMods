import { ActionIcon, Button, Group, ScrollArea, Stack, Text, Tooltip } from '@mantine/core'
import {
  IconLayoutSidebarLeftCollapse,
  IconLayoutSidebarLeftExpand,
  IconPlus,
} from '@tabler/icons-react'
import { useEffect, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { useGroupFilterStore } from '../../shared/group-filter-store'
import { CollapsedGroups } from './CollapsedGroups'
import { GroupRow } from './GroupRow'
import { GroupsMenu } from './GroupsMenu'
import { useGroupPrefs } from './group-prefs-store'
import { groupItems } from './group-state'
import classes from './groups.module.css'
import { openGroupDialog } from './profile-modals'
import { useProfiles } from './queries'
import type { GroupItem, GroupsSidebarProps } from './types'
import { useGroupSwitch } from './use-group-switch'

function sameKeys(a: readonly string[], b: readonly string[]): boolean {
  return a.length === b.length && a.every((key, index) => key === b[index])
}

// Groups beside the mod list: one switch per group, the name narrows the list to its mods.
export function GroupsSidebar({ rows, footer }: GroupsSidebarProps) {
  const { t, i18n } = useTranslation('profiles')
  const profiles = useProfiles()
  const view = profiles.data ?? undefined
  const readOnly = view?.readOnly ?? true
  const collapsed = useGroupPrefs((state) => state.collapsed)
  const setCollapsed = useGroupPrefs((state) => state.setCollapsed)
  const shown = useGroupFilterStore((state) => state.group)
  const { showGroup, clearGroup } = useGroupFilterStore.getState()
  const { switchGroup, busyId } = useGroupSwitch()
  const unnamed = t(($) => $.unnamed)
  const items = useMemo(
    () => groupItems(view, rows, i18n.language, unnamed),
    [view, rows, i18n.language, unnamed],
  )

  // The list filter follows edits of its group and ends when the group is deleted.
  useEffect(() => {
    if (shown === null || !view) return
    const item = items.find((candidate) => candidate.summary.profile.id === shown.id)
    if (!item) clearGroup()
    else if (item.title !== shown.name || !sameKeys(item.onKeys, shown.keys)) {
      showGroup({ id: shown.id, name: item.title, keys: item.onKeys })
    }
  }, [items, shown, view, showGroup, clearGroup])

  const onSwitch = (item: GroupItem) => switchGroup(item.summary.profile, item.lit ? 'off' : 'on')
  const onShow = (item: GroupItem) => {
    const { id } = item.summary.profile
    if (shown?.id === id) clearGroup()
    else showGroup({ id, name: item.title, keys: item.onKeys })
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
        <CollapsedGroups items={items} busyId={busyId} readOnly={readOnly} onSwitch={onSwitch} />
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
                active={shown?.id === item.summary.profile.id}
                busy={busyId === item.summary.profile.id}
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
