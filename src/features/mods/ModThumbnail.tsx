import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { ItemPicture } from '../../shared/components/ItemPicture'
import { useModsUiStore } from './mods-ui-store'
import { useModItems } from './queries'
import { listPicture } from './replaces-model'
import type { ModRowCellProps } from './types'

const SIZE = 40

// Virtual rows mount this only on screen, so a list of 150 mods loads what it shows.
export function ModThumbnail({ row }: ModRowCellProps) {
  const { t, i18n } = useTranslation('mods')
  const items = useModItems(row.modRef, i18n.language).data
  const character = useModsUiStore((state) => state.filter.characters[0] ?? null)
  const id = useMemo(() => listPicture(items, character), [items, character])

  return (
    <ItemPicture
      id={id}
      label={t(($) => $.cover.alt, { name: row.name })}
      width={SIZE}
      height={SIZE}
    />
  )
}
