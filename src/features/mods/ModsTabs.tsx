import { Badge, Tabs } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { ArchivesList } from './ArchivesList'
import { IssuesList } from './IssuesList'
import { countIssues } from './issue-count'
import { ModsListTab } from './ModsListTab'
import classes from './mods.module.css'
import { useModsUiStore } from './mods-ui-store'
import { useRepairList } from './queries'
import { RepairTab } from './RepairTab'
import type { ModsTab, ModsTabsProps } from './types'
import { UnmanagedList } from './UnmanagedList'

function isModsTab(value: string | null): value is ModsTab {
  return (
    value === 'mods' ||
    value === 'archives' ||
    value === 'unmanaged' ||
    value === 'issues' ||
    value === 'repair'
  )
}

export function ModsTabs({ snapshot, paks, searchRef }: ModsTabsProps) {
  const { t } = useTranslation('mods')
  const tab = useModsUiStore((state) => state.tab)
  const setTab = useModsUiStore((state) => state.setTab)
  const repair = useRepairList()
  const repairCount = repair.data?.items.length ?? 0
  const counts: Record<ModsTab, number> = {
    mods: snapshot.rows.length,
    archives: snapshot.archives.length,
    unmanaged: snapshot.unmanaged.length,
    issues: countIssues(snapshot.rows, snapshot.roots, snapshot.leftovers),
    repair: repairCount,
  }
  // The Repair tab only shows while there is something to repair, or while it is open.
  const tabs: readonly ModsTab[] =
    repairCount > 0 || tab === 'repair'
      ? ['mods', 'archives', 'unmanaged', 'issues', 'repair']
      : ['mods', 'archives', 'unmanaged', 'issues']

  return (
    <Tabs
      value={tab}
      onChange={(value) => {
        if (isModsTab(value)) setTab(value)
      }}
      keepMounted={false}
      className={classes.tabs}
    >
      <Tabs.List>
        {tabs.map((id) => (
          <Tabs.Tab
            key={id}
            value={id}
            rightSection={
              <Badge size="sm" variant="light" circle={false}>
                {counts[id]}
              </Badge>
            }
          >
            {t(($) => $.tabs[id])}
          </Tabs.Tab>
        ))}
      </Tabs.List>
      <Tabs.Panel value="mods" className={classes.tabPanel}>
        <ModsListTab rows={snapshot.rows} paks={paks} searchRef={searchRef} />
      </Tabs.Panel>
      <Tabs.Panel value="archives" className={classes.tabPanel}>
        <ArchivesList archives={snapshot.archives} />
      </Tabs.Panel>
      <Tabs.Panel value="unmanaged" className={classes.tabPanel}>
        <UnmanagedList entries={snapshot.unmanaged} />
      </Tabs.Panel>
      <Tabs.Panel value="issues" className={classes.tabPanel}>
        <IssuesList rows={snapshot.rows} roots={snapshot.roots} leftovers={snapshot.leftovers} />
      </Tabs.Panel>
      <Tabs.Panel value="repair" className={classes.tabPanel}>
        <RepairTab list={repair.data ?? undefined} />
      </Tabs.Panel>
    </Tabs>
  )
}
