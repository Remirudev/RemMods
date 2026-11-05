import { Center, SegmentedControl, VisuallyHidden } from '@mantine/core'
import { IconLayoutGrid, IconList } from '@tabler/icons-react'
import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { useModsViewStore } from './mods-view-store'
import type { ModsView } from './types'

function isView(value: string): value is ModsView {
  return value === 'table' || value === 'grid'
}

export function ModsViewSwitch() {
  const { t } = useTranslation('mods')
  const view = useModsViewStore((state) => state.view)
  const setView = useModsViewStore((state) => state.setView)
  const option = (value: ModsView, label: string, icon: ReactNode) => ({
    value,
    label: (
      <Center>
        {icon}
        <VisuallyHidden>{label}</VisuallyHidden>
      </Center>
    ),
  })

  return (
    <SegmentedControl
      size="xs"
      aria-label={t(($) => $.view.label)}
      value={view}
      onChange={(value) => {
        if (isView(value)) setView(value)
      }}
      data={[
        option(
          'table',
          t(($) => $.view.table),
          <IconList size={16} stroke={1.8} />,
        ),
        option(
          'grid',
          t(($) => $.view.grid),
          <IconLayoutGrid size={16} stroke={1.8} />,
        ),
      ]}
    />
  )
}
