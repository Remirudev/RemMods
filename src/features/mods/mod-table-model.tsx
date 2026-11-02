import { createColumnHelper } from '@tanstack/react-table'
import type { TFunction } from 'i18next'
import type { ModRow } from '../../lib/ipc/types'
import { formatSize } from '../../shared/format-size'
import { ConflictBadge } from './ConflictBadge'
import { ContentConflictBadge } from './ContentConflictBadge'
import { COLUMN_SIZES } from './column-layout'
import { compareMods, modCollator } from './compare-mods'
import { outfitConflicts } from './content-conflicts'
import { displayState } from './display-state'
import { IssueBadge } from './IssueBadge'
import { modInfoText } from './mod-info-text'
import type { modTableFeatures } from './mod-table-features'
import { NameCell } from './NameCell'
import { RowMenu } from './RowMenu'
import { StateBadge } from './StateBadge'
import { ToggleCell } from './ToggleCell'
import type { ManageRows, ModColumnId, ToggleRows } from './types'

const columnHelper = createColumnHelper<typeof modTableFeatures, ModRow>()

// Descending first: the biggest, most conflicted or most troubled mods are what people look for.
const DESC_FIRST: ReadonlySet<ModColumnId> = new Set<ModColumnId>([
  'size',
  'conflicts',
  'contentConflicts',
  'issues',
])

const TOGGLE_COLUMN_WIDTH = 56
const MENU_COLUMN_WIDTH = 52

// Cell functions are component types for the table, so the columns are rebuilt per language.
// `onToggle` and `onManage` must be stable, or the columns (and every switch) are rebuilt on each
// render.
export function buildModColumns(
  t: TFunction<'mods'>,
  locale: string,
  onToggle: ToggleRows,
  onManage: ManageRows,
) {
  const collator = modCollator(locale)
  const sort = (column: ModColumnId) => (a: { original: ModRow }, b: { original: ModRow }) =>
    compareMods(a.original, b.original, column, collator)
  const base = (id: ModColumnId) => ({
    id,
    header: t(($) => $.columns[id]),
    sortFn: sort(id),
    sortDescFirst: DESC_FIRST.has(id),
    ...COLUMN_SIZES[id],
    enableHiding: id !== 'name',
    meta: { resizable: true },
  })

  return columnHelper.columns([
    columnHelper.display({
      id: 'toggle',
      header: t(($) => $.columns.toggle),
      enableSorting: false,
      enableHiding: false,
      size: TOGGLE_COLUMN_WIDTH,
      cell: ({ row }) => <ToggleCell row={row.original} onToggle={onToggle} />,
    }),
    columnHelper.accessor((row) => row.name, {
      ...base('name'),
      cell: ({ row }) => <NameCell row={row.original} />,
    }),
    columnHelper.accessor((row) => displayState(row), {
      ...base('state'),
      cell: ({ row }) => <StateBadge row={row.original} />,
    }),
    columnHelper.accessor((row) => row.kind, {
      ...base('kind'),
      cell: ({ row }) => t(($) => $.kinds[row.original.kind]),
    }),
    columnHelper.accessor((row) => row.info.version, {
      ...base('version'),
      cell: ({ row }) => modInfoText(row.original.info, 'version', t),
    }),
    columnHelper.accessor((row) => row.info.category, {
      ...base('category'),
      cell: ({ row }) => modInfoText(row.original.info, 'category', t),
    }),
    columnHelper.accessor((row) => row.sizeBytes, {
      ...base('size'),
      meta: { align: 'right', resizable: true },
      cell: ({ row }) => formatSize(row.original.sizeBytes, locale) ?? t(($) => $.size.unknown),
    }),
    columnHelper.accessor((row) => row.conflictsWith.length, {
      ...base('conflicts'),
      cell: ({ row }) => <ConflictBadge row={row.original} />,
    }),
    columnHelper.accessor((row) => outfitConflicts(row).length, {
      ...base('contentConflicts'),
      cell: ({ row }) => <ContentConflictBadge row={row.original} />,
    }),
    columnHelper.accessor((row) => row.issues.length, {
      ...base('issues'),
      cell: ({ row }) => <IssueBadge row={row.original} />,
    }),
    columnHelper.display({
      id: 'menu',
      header: '',
      enableSorting: false,
      enableHiding: false,
      size: MENU_COLUMN_WIDTH,
      cell: ({ row }) => <RowMenu row={row.original} onManage={onManage} />,
    }),
  ])
}
