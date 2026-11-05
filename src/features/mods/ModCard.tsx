import { Group, Table, Text } from '@mantine/core'
import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { ItemPicture } from '../../shared/components/ItemPicture'
import { ConflictBadge } from './ConflictBadge'
import { ContentConflictBadge } from './ContentConflictBadge'
import { characterSummary, slotLabel } from './content-hints'
import { displayState } from './display-state'
import { IssueBadge } from './IssueBadge'
import classes from './mods.module.css'
import { useModsUiStore } from './mods-ui-store'
import { useModItems } from './queries'
import { RowMenu } from './RowMenu'
import { listPicture, replacedNames } from './replaces-model'
import { StateBadge } from './StateBadge'
import { ToggleCell } from './ToggleCell'
import type { ModCardProps } from './types'

const PICTURE_HEIGHT = 170

export function ModCard({
  row,
  index,
  active,
  selected,
  onClick,
  onToggle,
  onManage,
}: ModCardProps) {
  const { t, i18n } = useTranslation('mods')
  const locale = i18n.language
  const items = useModItems(row.modRef, locale).data
  const character = useModsUiStore((state) => state.filter.characters[0] ?? null)
  const picture = useMemo(() => listPicture(items, character), [items, character])
  const replaces = useMemo(() => {
    const names = replacedNames(items?.items ?? [], character, locale)
    return names.length > 0 ? names : row.content.slots.map((hint) => slotLabel(hint, t))
  }, [items, character, locale, row.content.slots, t])
  const off = displayState(row) === 'disabled'

  return (
    <Table.Td
      id={`mods-card-${index}`}
      data-testid="mods-card"
      className={classes.card}
      aria-selected={selected}
      mod={{ active, selected, off }}
      onClick={(event) => onClick(index, event)}
    >
      <div className={classes.cardPicture}>
        <ItemPicture
          id={picture}
          label={t(($) => $.cover.alt, { name: row.name })}
          width="100%"
          height={PICTURE_HEIGHT}
        />
      </div>
      <div className={classes.cardMenu}>
        <RowMenu row={row} onManage={onManage} />
      </div>
      <div className={classes.cardBody}>
        <Group gap="xs" justify="space-between" wrap="nowrap">
          <Text size="sm" fw={500} truncate="end" title={row.name}>
            <bdi>{row.name}</bdi>
          </Text>
          <ToggleCell row={row} onToggle={onToggle} />
        </Group>
        <Text size="xs" truncate="end" title={replaces.join(', ')}>
          <bdi>{replaces.join(', ')}</bdi>
        </Text>
        <Text size="xs" c="var(--qc-text-2)" truncate="end">
          <bdi>{characterSummary(row.content.characters, t, locale)}</bdi>
        </Text>
        <Group gap={4} wrap="nowrap" className={classes.cardBadges}>
          <StateBadge row={row} />
          <ConflictBadge row={row} />
          <ContentConflictBadge row={row} />
          <IssueBadge row={row} />
        </Group>
      </div>
    </Table.Td>
  )
}
