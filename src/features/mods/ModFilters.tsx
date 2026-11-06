import {
  ActionIcon,
  Checkbox,
  Group,
  Select,
  Text,
  TextInput,
  Tooltip,
  VisuallyHidden,
} from '@mantine/core'
import { IconFilterOff, IconSearch } from '@tabler/icons-react'
import { useId, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import type { ModKind } from '../../lib/ipc/types'
import { useGroupFilterStore } from '../../shared/group-filter-store'
import { ColumnsMenu } from './ColumnsMenu'
import { characterLabel } from './content-hints'
import { FilterMultiSelect } from './FilterMultiSelect'
import { authorChoices, characterHints, isFilterActive } from './filter-mods'
import { ModSortMenu } from './ModSortMenu'
import { ModsViewSwitch } from './ModsViewSwitch'
import { modInfoText } from './mod-info-text'
import classes from './mods.module.css'
import { useModsUiStore } from './mods-ui-store'
import { useModsViewStore } from './mods-view-store'
import type { ModFiltersProps, StateFilter } from './types'

const STATE_OPTIONS: readonly StateFilter[] = [
  'all',
  'enabled',
  'partial',
  'disabledLoaded',
  'disabled',
  'noPakFiles',
  'unknown',
]
const KIND_OPTIONS: readonly ModKind[] = ['standard', 'legacy', 'logic', 'loose']

function sortedByLabel<T extends { label: string }>(items: T[], locale: string): T[] {
  return items.sort((a, b) => a.label.localeCompare(b.label, locale))
}

export function ModFilters({ rows, shown, total, searchRef }: ModFiltersProps) {
  const { t, i18n } = useTranslation('mods')
  const contentHintId = useId()
  const contentHint = t(($) => $.filters.contentConflictsHint)
  const clearLabel = t(($) => $.filters.clear)
  const filter = useModsUiStore((state) => state.filter)
  const groups = useGroupFilterStore((state) => state.groups)
  const pickedGroups = useGroupFilterStore((state) => state.picked)
  const view = useModsViewStore((state) => state.view)
  const locale = i18n.language
  const characters = useMemo(
    () =>
      sortedByLabel(
        characterHints(rows).map((hint) => ({ value: hint.code, label: characterLabel(hint, t) })),
        locale,
      ),
    [rows, t, locale],
  )
  const authors = useMemo(
    () =>
      sortedByLabel(
        authorChoices(rows, (info) => modInfoText(info, 'author', t)),
        locale,
      ),
    [rows, t, locale],
  )
  const {
    setText,
    setStateFilter,
    setKinds,
    setCharacters,
    setAuthors,
    setConflictsOnly,
    setContentConflictsOnly,
    setIssuesOnly,
    clearFilters,
  } = useModsUiStore.getState()
  const { pickGroups } = useGroupFilterStore.getState()

  return (
    <div className={classes.filters}>
      <div className={classes.filterFields}>
        <TextInput
          ref={searchRef}
          className={classes.searchField}
          label={t(($) => $.filters.search)}
          placeholder={t(($) => $.filters.searchPlaceholder)}
          leftSection={<IconSearch size={16} stroke={1.8} />}
          value={filter.text}
          onChange={(event) => setText(event.currentTarget.value)}
        />
        <Select
          className={classes.filterField}
          label={t(($) => $.filters.state)}
          allowDeselect={false}
          data={STATE_OPTIONS.map((value) => ({
            value,
            label: t(($) => $.filters.states[value]),
          }))}
          value={filter.state}
          onChange={(value) => {
            const next = STATE_OPTIONS.find((option) => option === value)
            if (next) setStateFilter(next)
          }}
        />
        <FilterMultiSelect
          label={t(($) => $.filters.kinds)}
          data={KIND_OPTIONS.map((value) => ({ value, label: t(($) => $.kinds[value]) }))}
          value={filter.kinds}
          onChange={(values) => setKinds(KIND_OPTIONS.filter((kind) => values.includes(kind)))}
        />
        <FilterMultiSelect
          searchable
          label={t(($) => $.filters.characters)}
          data={characters}
          value={filter.characters}
          onChange={setCharacters}
        />
        <FilterMultiSelect
          searchable
          label={t(($) => $.filters.authors)}
          data={authors}
          value={filter.authors}
          onChange={setAuthors}
        />
        <FilterMultiSelect
          searchable
          label={t(($) => $.filters.groups)}
          data={groups.map((group) => ({ value: group.id, label: group.name }))}
          value={pickedGroups}
          onChange={(ids) => pickGroups(ids, ids.at(-1) ?? null)}
        />
      </div>
      <div className={classes.filterRow}>
        <Group gap="md" wrap="nowrap" className={classes.filterChecks}>
          <Checkbox
            label={t(($) => $.filters.conflictsOnly)}
            checked={filter.conflictsOnly}
            onChange={(event) => setConflictsOnly(event.currentTarget.checked)}
          />
          <Tooltip
            label={contentHint}
            multiline
            maw={320}
            events={{ hover: true, focus: true, touch: false }}
          >
            <div className={classes.filterCheck}>
              <Checkbox
                label={t(($) => $.filters.contentConflictsOnly)}
                checked={filter.contentConflictsOnly}
                onChange={(event) => setContentConflictsOnly(event.currentTarget.checked)}
                aria-describedby={contentHintId}
              />
              <VisuallyHidden id={contentHintId}>{contentHint}</VisuallyHidden>
            </div>
          </Tooltip>
          <Checkbox
            label={t(($) => $.filters.issuesOnly)}
            checked={filter.issuesOnly}
            onChange={(event) => setIssuesOnly(event.currentTarget.checked)}
          />
        </Group>
        <Group gap="sm" wrap="nowrap" className={classes.filterStatus}>
          <Text
            size="sm"
            c="var(--qc-text-2)"
            className={classes.filterCount}
            aria-live="polite"
            data-testid="mods-count"
          >
            {t(($) => $.filters.count, { count: shown, total })}
          </Text>
          {(isFilterActive(filter) || pickedGroups.length > 0) && (
            <Tooltip label={clearLabel}>
              <ActionIcon
                variant="subtle"
                color="gray"
                size={30}
                aria-label={clearLabel}
                onClick={clearFilters}
              >
                <IconFilterOff size={18} stroke={1.8} />
              </ActionIcon>
            </Tooltip>
          )}
          {view === 'table' ? <ColumnsMenu /> : <ModSortMenu />}
          <ModsViewSwitch />
        </Group>
      </div>
    </div>
  )
}
