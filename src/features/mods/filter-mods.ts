import type { ModRow } from '../../lib/ipc/types'
import { outfitConflicts } from './content-conflicts'
import { displayState } from './display-state'
import type { KindLabel, ModFilter } from './types'

export const EMPTY_FILTER: ModFilter = {
  text: '',
  state: 'all',
  kinds: [],
  conflictsOnly: false,
  contentConflictsOnly: false,
  issuesOnly: false,
}

export function isFilterActive(filter: ModFilter): boolean {
  return (
    filter.text.trim() !== '' ||
    filter.state !== 'all' ||
    filter.kinds.length > 0 ||
    filter.conflictsOnly ||
    filter.contentConflictsOnly ||
    filter.issuesOnly
  )
}

function matchesText(row: ModRow, needle: string, kindLabel: KindLabel, locale: string): boolean {
  const haystack = [row.name, row.key, row.info.category, row.info.author, kindLabel(row.kind)]
  for (const hint of row.content.characters) {
    haystack.push(hint.code)
    if (hint.name !== null) haystack.push(hint.name)
  }
  return haystack.some((value) => value.toLocaleLowerCase(locale).includes(needle))
}

// The state filter compares the display state, so `disabled` leaves out LogicMods folders that
// are disabled by name only and still loaded.
export function filterMods(
  rows: readonly ModRow[],
  filter: ModFilter,
  kindLabel: KindLabel,
  locale: string,
): ModRow[] {
  const needle = filter.text.trim().toLocaleLowerCase(locale)
  return rows.filter(
    (row) =>
      (filter.state === 'all' || displayState(row) === filter.state) &&
      (filter.kinds.length === 0 || filter.kinds.includes(row.kind)) &&
      (!filter.conflictsOnly || row.conflictsWith.length > 0) &&
      (!filter.contentConflictsOnly || outfitConflicts(row).length > 0) &&
      (!filter.issuesOnly || row.issues.length > 0) &&
      (needle === '' || matchesText(row, needle, kindLabel, locale)),
  )
}
