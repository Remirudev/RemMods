import {
  Badge,
  Button,
  Checkbox,
  CloseButton,
  Group,
  MultiSelect,
  Select,
  Text,
  TextInput,
  Tooltip,
  VisuallyHidden,
} from '@mantine/core'
import { IconSearch } from '@tabler/icons-react'
import { useId } from 'react'
import { useTranslation } from 'react-i18next'
import type { ModKind } from '../../lib/ipc/types'
import { useGroupFilterStore } from '../../shared/group-filter-store'
import { ColumnsMenu } from './ColumnsMenu'
import { isFilterActive } from './filter-mods'
import { useModsUiStore } from './mods-ui-store'
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

export function ModFilters({ shown, total, searchRef }: ModFiltersProps) {
  const { t } = useTranslation('mods')
  const contentHintId = useId()
  const contentHint = t(($) => $.filters.contentConflictsHint)
  const filter = useModsUiStore((state) => state.filter)
  const group = useGroupFilterStore((state) => state.group)
  const clearGroup = useGroupFilterStore((state) => state.clearGroup)
  const {
    setText,
    setStateFilter,
    setKinds,
    setConflictsOnly,
    setContentConflictsOnly,
    setIssuesOnly,
    clearFilters,
  } = useModsUiStore.getState()

  return (
    <Group gap="md" align="flex-end" wrap="wrap">
      <TextInput
        ref={searchRef}
        w={280}
        label={t(($) => $.filters.search)}
        placeholder={t(($) => $.filters.searchPlaceholder)}
        leftSection={<IconSearch size={16} stroke={1.8} />}
        value={filter.text}
        onChange={(event) => setText(event.currentTarget.value)}
      />
      <Select
        w={200}
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
      <MultiSelect
        w={220}
        label={t(($) => $.filters.kinds)}
        data={KIND_OPTIONS.map((value) => ({ value, label: t(($) => $.kinds[value]) }))}
        value={[...filter.kinds]}
        onChange={(values) => setKinds(KIND_OPTIONS.filter((kind) => values.includes(kind)))}
      />
      <Group gap="md" h={36}>
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
          <div>
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
      <Group gap="sm" h={36} ml="auto">
        {group && (
          <Badge
            variant="light"
            size="lg"
            tt="none"
            pr={4}
            maw={260}
            data-testid="group-filter"
            rightSection={
              <CloseButton
                size="xs"
                aria-label={t(($) => $.filters.clearGroup)}
                onClick={clearGroup}
              />
            }
          >
            <bdi>{t(($) => $.filters.group, { name: group.name })}</bdi>
          </Badge>
        )}
        <Text size="sm" c="var(--qc-text-2)" aria-live="polite" data-testid="mods-count">
          {t(($) => $.filters.count, { count: shown, total })}
        </Text>
        {(isFilterActive(filter) || group !== null) && (
          <Button variant="subtle" size="compact-sm" onClick={clearFilters}>
            {t(($) => $.filters.clear)}
          </Button>
        )}
        <ColumnsMenu />
      </Group>
    </Group>
  )
}
