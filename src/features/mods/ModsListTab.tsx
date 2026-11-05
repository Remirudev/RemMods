import { Button, Stack, Text } from '@mantine/core'
import { IconPackages } from '@tabler/icons-react'
import { useCallback, useEffect, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { EmptyState } from '../../shared/components/EmptyState'
import { useGroupFilterStore } from '../../shared/group-filter-store'
import { ActionBar } from './ActionBar'
import { filterMods } from './filter-mods'
import { ModFilters } from './ModFilters'
import { ModGrid } from './ModGrid'
import { ModTable } from './ModTable'
import { rowSubject } from './manage-model'
import { selectedRows } from './mod-actions'
import { useModsUiStore } from './mods-ui-store'
import { useModsViewStore } from './mods-view-store'
import { pruneSelection } from './selection'
import { rowsToChange, toggleTargets } from './toggle-model'
import type { BulkAction, ManageRows, ModsListTabProps, ToggleRows } from './types'
import { useBulkActions } from './use-bulk-actions'
import { useManageActions } from './use-manage-actions'
import { useToggleMods } from './use-toggle-mods'

export function ModsListTab({ rows, paks, searchRef }: ModsListTabProps) {
  const { t, i18n } = useTranslation('mods')
  const filter = useModsUiStore((state) => state.filter)
  const clearFilters = useModsUiStore((state) => state.clearFilters)
  const openDetails = useModsUiStore((state) => state.openDetails)
  const groupKeys = useGroupFilterStore((state) => state.group?.keys ?? null)
  const view = useModsViewStore((state) => state.view)
  const toggle = useToggleMods()
  const { mutate } = toggle
  const manage = useManageActions()
  const bulk = useBulkActions()
  const locale = i18n.language
  const visible = useMemo(
    () => filterMods(rows, filter, (kind) => t(($) => $.kinds[kind]), locale, groupKeys),
    [rows, filter, t, locale, groupKeys],
  )

  // Rows hidden by a filter or gone from the disk leave the selection, so a bulk action never
  // reaches a row the user cannot see.
  useEffect(() => {
    const { selection, anchor, setSelection } = useModsUiStore.getState()
    const pruned = pruneSelection(
      selection,
      visible.map((row) => row.modRef),
    )
    if (pruned === selection) return
    const keepsAnchor = anchor !== null && pruned.includes(anchor)
    setSelection(pruned, keepsAnchor ? anchor : null)
  }, [visible])

  const toggleRows = useCallback<ToggleRows>(
    (targets, enable) => {
      const changing = rowsToChange(targets, enable)
      if (changing.length === 0) return
      mutate({ targets: toggleTargets(changing), enable, applyWhileRunning: false })
    },
    [mutate],
  )

  const manageRow = useCallback<ManageRows>(
    (action, row) => {
      const subject = rowSubject(row)
      if (action === 'rename') manage.rename(subject)
      else if (action === 'trash') manage.trash(subject)
      else manage.remove(subject)
    },
    [manage],
  )

  const bulkRows = useCallback(
    (action: BulkAction) => {
      const selected = selectedRows(visible, useModsUiStore.getState().selection)
      if (action === 'trash') bulk.trash(selected)
      else bulk.remove(selected)
    },
    [bulk, visible],
  )

  if (rows.length === 0) {
    return (
      <EmptyState
        icon={<IconPackages size={28} stroke={1.6} />}
        title={t(($) => $.empty.title)}
        description={t(($) => $.empty.description, { paks })}
      />
    )
  }

  const List = view === 'grid' ? ModGrid : ModTable

  return (
    <Stack gap="md" flex={1} mih={0}>
      <ModFilters rows={rows} shown={visible.length} total={rows.length} searchRef={searchRef} />
      <ActionBar rows={visible} onToggle={toggleRows} onTrash={bulk.trash} onDelete={bulk.remove} />
      {visible.length === 0 ? (
        <Stack gap="sm" align="flex-start" py="lg">
          <Text size="sm">{t(($) => $.table.noMatch)}</Text>
          <Button variant="default" onClick={clearFilters}>
            {t(($) => $.filters.clear)}
          </Button>
        </Stack>
      ) : (
        <List
          rows={visible}
          onOpen={openDetails}
          onToggle={toggleRows}
          onManage={manageRow}
          onBulk={bulkRows}
        />
      )}
    </Stack>
  )
}
