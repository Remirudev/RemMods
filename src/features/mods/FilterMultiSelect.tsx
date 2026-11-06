import { Combobox, MultiSelect } from '@mantine/core'
import { useTranslation } from 'react-i18next'
import classes from './mods.module.css'
import type { FilterMultiSelectProps } from './types'

// One line whatever is picked: the first pick and how many more, so a filter never grows its row.
export function FilterMultiSelect({
  label,
  data,
  value,
  searchable,
  onChange,
}: FilterMultiSelectProps) {
  const { t } = useTranslation('mods')
  const [first] = value
  const more = value.length - 1
  const picked = first !== undefined

  return (
    <MultiSelect
      className={classes.filterField}
      classNames={{ pillsList: classes.filterPicks }}
      label={label}
      data={[...data]}
      value={[...value]}
      searchable={searchable}
      rightSection={
        picked ? (
          <Combobox.ClearButton
            aria-hidden={false}
            aria-label={t(($) => $.filters.clearField, { name: label })}
            tabIndex={0}
            onClear={() => onChange([])}
          />
        ) : undefined
      }
      rightSectionPointerEvents={picked ? 'all' : 'none'}
      onChange={onChange}
      renderPill={({ option, value: pick }) =>
        pick === first ? (
          <span className={classes.filterPick}>
            <bdi className={classes.filterPickName}>{option?.label ?? pick}</bdi>
            {more > 0 && <span className={classes.filterPickMore}>+{more}</span>}
          </span>
        ) : null
      }
    />
  )
}
