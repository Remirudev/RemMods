import { Badge, Tabs } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { ArchivesList } from './ArchivesList'
import { IssuesList } from './IssuesList'
import { countIssues } from './issue-count'
import { ModsListTab } from './ModsListTab'
import classes from './mods.module.css'
import { useModsUiStore } from './mods-ui-store'
import type { ModsTab, ModsTabsProps } from './types'
import { UnmanagedList } from './UnmanagedList'

function isModsTab(value: string | null): value is ModsTab {
  return value === 'mods' || value === 'archives' || value === 'unmanaged' || value === 'issues'
}

export function ModsTabs({ snapshot, paks, searchRef }: ModsTabsProps) {
  const { t } = useTranslation('mods')
  const tab = useModsUiStore((state) => state.tab)
  const setTab = useModsUiStore((state) => state.setTab)
  const counts: Record<ModsTab, number> = {
    mods: snapshot.rows.length,
    archives: snapshot.archives.length,
    unmanaged: snapshot.unmanaged.length,
    issues: countIssues(snapshot.rows, snapshot.roots, snapshot.leftovers),
  }
  const tabs: readonly ModsTab[] = ['mods', 'archives', 'unmanaged', 'issues']

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
    </Tabs>
  )
}
