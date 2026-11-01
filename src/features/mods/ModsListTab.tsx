import { Button, Stack, Text } from '@mantine/core'
import { IconPackages } from '@tabler/icons-react'
import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { EmptyState } from '../../shared/components/EmptyState'
import { filterMods } from './filter-mods'
import { ModFilters } from './ModFilters'
import { ModTable } from './ModTable'
import { useModsUiStore } from './mods-ui-store'
import type { ModsListTabProps } from './types'

export function ModsListTab({ rows, paks, searchRef }: ModsListTabProps) {
  const { t, i18n } = useTranslation('mods')
  const filter = useModsUiStore((state) => state.filter)
  const clearFilters = useModsUiStore((state) => state.clearFilters)
  const openDetails = useModsUiStore((state) => state.openDetails)
  const locale = i18n.language
  const visible = useMemo(
    () => filterMods(rows, filter, (kind) => t(($) => $.kinds[kind]), locale),
    [rows, filter, t, locale],
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
      {visible.length === 0 ? (
        <Stack gap="sm" align="flex-start" py="lg">
          <Text size="sm">{t(($) => $.table.noMatch)}</Text>
          <Button variant="default" onClick={clearFilters}>
            {t(($) => $.filters.clear)}
          </Button>
        </Stack>
      ) : (
        <ModTable rows={visible} onOpen={openDetails} />
      )}
    </Stack>
  )
}
