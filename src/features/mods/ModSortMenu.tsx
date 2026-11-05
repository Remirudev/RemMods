import { ActionIcon, Group, Select, Tooltip } from '@mantine/core'
import { IconSortAscending, IconSortDescending } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import { MOD_COLUMN_IDS } from './column-layout'
import { useModsUiStore } from './mods-ui-store'

// The grid has no column headers to click: this sets the same sort the table uses.
export function ModSortMenu() {
  const { t } = useTranslation('mods')
  const sorting = useModsUiStore((state) => state.sorting)
  const setSorting = useModsUiStore((state) => state.setSorting)
  const sort = sorting[0]
  const descending = sort?.desc ?? false
  const label = descending ? t(($) => $.grid.descending) : t(($) => $.grid.ascending)
  const Icon = descending ? IconSortDescending : IconSortAscending

  return (
    <Group gap={4} wrap="nowrap">
      <Select
        size="xs"
        w={150}
        aria-label={t(($) => $.grid.sortBy)}
        allowDeselect={false}
        data={MOD_COLUMN_IDS.map((id) => ({ value: id, label: t(($) => $.columns[id]) }))}
        value={sort?.id ?? 'name'}
        onChange={(id) => {
          if (id !== null) setSorting([{ id, desc: descending }])
        }}
      />
      <Tooltip label={label}>
        <ActionIcon
          variant="default"
          size={30}
          aria-label={label}
          onClick={() => setSorting([{ id: sort?.id ?? 'name', desc: !descending }])}
        >
          <Icon size={16} stroke={1.8} />
        </ActionIcon>
      </Tooltip>
    </Group>
  )
}
