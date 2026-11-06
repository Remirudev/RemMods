import type { CharacterHint, ModInfo, ModRow } from '../../lib/ipc/types'
import { outfitConflicts } from '../../shared/outfit-conflicts'
import { displayState } from './display-state'
import type { AuthorChoice, KindLabel, ModFilter } from './types'

export const EMPTY_FILTER: ModFilter = {
  text: '',
  state: 'all',
  kinds: [],
  characters: [],
  authors: [],
  conflictsOnly: false,
  contentConflictsOnly: false,
  issuesOnly: false,
}

export function isFilterActive(filter: ModFilter): boolean {
  return (
    filter.text.trim() !== '' ||
    filter.state !== 'all' ||
    filter.kinds.length > 0 ||
    filter.characters.length > 0 ||
    filter.authors.length > 0 ||
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

// A mod for everyone is also a mod for the chosen character.
function matchesCharacters(row: ModRow, characters: readonly string[]): boolean {
  if (characters.length === 0) return true
  return row.content.characters.some((hint) => hint.common || characters.includes(hint.code))
}

// Every character some mod names, for the filter's list; the "everyone" code is no choice.
export function characterHints(rows: readonly ModRow[]): CharacterHint[] {
  const byCode = new Map(
    rows.flatMap((row) => row.content.characters.map((hint) => [hint.code, hint] as const)),
  )
  return [...byCode.values()].filter((hint) => !hint.common)
}

// Every author among the rows, once. Values stay raw like the filter's comparison; `label` is what
// the list shows, so the "unknown" default reads translated.
export function authorChoices(
  rows: readonly ModRow[],
  label: (info: ModInfo) => string,
): AuthorChoice[] {
  const byAuthor = new Map(rows.map((row) => [row.info.author, label(row.info)]))
  return [...byAuthor].map(([value, text]) => ({ value, label: text }))
}

// The state filter compares the display state, so `disabled` leaves out LogicMods folders that
// are disabled by name only and still loaded. `groupKeys` (a group picked in the sidebar) keeps
// only the rows with one of those keys; null keeps every row.
export function filterMods(
  rows: readonly ModRow[],
  filter: ModFilter,
  kindLabel: KindLabel,
  locale: string,
  groupKeys: readonly string[] | null = null,
): ModRow[] {
  const needle = filter.text.trim().toLocaleLowerCase(locale)
  const inGroup = groupKeys === null ? null : new Set(groupKeys)
  return rows.filter(
    (row) =>
      (inGroup === null || inGroup.has(row.key)) &&
      (filter.state === 'all' || displayState(row) === filter.state) &&
      (filter.kinds.length === 0 || filter.kinds.includes(row.kind)) &&
      matchesCharacters(row, filter.characters) &&
      (filter.authors.length === 0 || filter.authors.includes(row.info.author)) &&
      (!filter.conflictsOnly || row.conflictsWith.length > 0) &&
      (!filter.contentConflictsOnly || outfitConflicts(row).length > 0) &&
      (!filter.issuesOnly || row.issues.length > 0) &&
      (needle === '' || matchesText(row, needle, kindLabel, locale)),
  )
}
