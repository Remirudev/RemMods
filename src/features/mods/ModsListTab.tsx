import { Button, Stack, Text } from '@mantine/core'
import { IconPackages } from '@tabler/icons-react'
import { useCallback, useEffect, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { EmptyState } from '../../shared/components/EmptyState'
import { ActionBar } from './ActionBar'
import { filterMods } from './filter-mods'
import { ModFilters } from './ModFilters'
import { ModTable } from './ModTable'
import { useModsUiStore } from './mods-ui-store'
import { pruneSelection } from './selection'
import { rowsToChange, toggleTargets } from './toggle-model'
import type { ModsListTabProps, ToggleRows } from './types'
import { useToggleMods } from './use-toggle-mods'

export function ModsListTab({ rows, paks, searchRef }: ModsListTabProps) {
  const { t, i18n } = useTranslation('mods')
  const filter = useModsUiStore((state) => state.filter)
  const clearFilters = useModsUiStore((state) => state.clearFilters)
  const openDetails = useModsUiStore((state) => state.openDetails)
  const toggle = useToggleMods()
  const { mutate } = toggle
  const locale = i18n.language
  const visible = useMemo(
    () => filterMods(rows, filter, (kind) => t(($) => $.kinds[kind]), locale),
    [rows, filter, t, locale],
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

  if (rows.length === 0) {
    return (
      <EmptyState
        icon={<IconPackages size={28} stroke={1.6} />}
        title={t(($) => $.empty.title)}
        description={t(($) => $.empty.description, { paks })}
      />
    )
  }

  return (
    <Stack gap="md" flex={1} mih={0}>
      <ModFilters shown={visible.length} total={rows.length} searchRef={searchRef} />
      <ActionBar rows={visible} onToggle={toggleRows} />
      {visible.length === 0 ? (
        <Stack gap="sm" align="flex-start" py="lg">
          <Text size="sm">{t(($) => $.table.noMatch)}</Text>
          <Button variant="default" onClick={clearFilters}>
            {t(($) => $.filters.clear)}
          </Button>
        </Stack>
      ) : (
        <ModTable rows={visible} onOpen={openDetails} onToggle={toggleRows} />
      )}
    </Stack>
  )
}
