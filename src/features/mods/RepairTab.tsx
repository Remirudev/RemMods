import { Stack } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import { ListSection } from './ListSection'
import { RepairItemRow } from './RepairItemRow'
import type { RepairTabProps } from './types'

export function RepairTab({ list }: RepairTabProps) {
  const { t } = useTranslation('mods')
  const items = list?.items ?? []

  return (
    <ListSection
      description={t(($) => $.repair.description)}
      empty={t(($) => $.repair.empty)}
      count={items.length}
    >
      <Stack gap="xs">
        {items.map((item) => (
          <RepairItemRow key={item.id} item={item} />
        ))}
      </Stack>
    </ListSection>
  )
}
