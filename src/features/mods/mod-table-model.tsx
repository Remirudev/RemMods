import {
  createColumnHelper,
  createSortedRowModel,
  metaHelper,
  rowSortingFeature,
  tableFeatures,
} from '@tanstack/react-table'
import type { TFunction } from 'i18next'
import type { ModRow } from '../../lib/ipc/types'
import { formatSize } from '../../shared/format-size'
import { ConflictBadge } from './ConflictBadge'
import { compareMods, modCollator } from './compare-mods'
import { displayState } from './display-state'
import { IssueBadge } from './IssueBadge'
import { modInfoText } from './mod-info-text'
import { NameCell } from './NameCell'
import { RowMenu } from './RowMenu'
import { StateBadge } from './StateBadge'
import { ToggleCell } from './ToggleCell'
import type { ManageRows, ModColumnId, ModColumnMeta, ModTableMeta, ToggleRows } from './types'

export const modTableFeatures = tableFeatures({
  rowSortingFeature,
  sortedRowModel: createSortedRowModel(),
  tableMeta: metaHelper<ModTableMeta>(),
  columnMeta: metaHelper<ModColumnMeta>(),
})

const columnHelper = createColumnHelper<typeof modTableFeatures, ModRow>()

// Descending first: the biggest, most conflicted or most troubled mods are what people look for.
const DESC_FIRST: ReadonlySet<ModColumnId> = new Set<ModColumnId>(['size', 'conflicts', 'issues'])

const COLUMN_WIDTHS: Record<ModColumnId, number | undefined> = {
  name: undefined,
  state: 140,
  kind: 110,
  version: 90,
  category: 130,
  size: 100,
  conflicts: 120,
  issues: 110,
}

export const TOGGLE_COLUMN_WIDTH = 56
export const MENU_COLUMN_WIDTH = 52

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
    meta: { width: COLUMN_WIDTHS[id] },
  })

  return columnHelper.columns([
    columnHelper.display({
      id: 'toggle',
      header: t(($) => $.columns.toggle),
      enableSorting: false,
      meta: { width: TOGGLE_COLUMN_WIDTH },
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
      meta: { width: COLUMN_WIDTHS.size, align: 'right' },
      cell: ({ row }) => formatSize(row.original.sizeBytes, locale) ?? t(($) => $.size.unknown),
    }),
    columnHelper.accessor((row) => row.conflictsWith.length, {
      ...base('conflicts'),
      cell: ({ row }) => <ConflictBadge row={row.original} />,
    }),
    columnHelper.accessor((row) => row.issues.length, {
      ...base('issues'),
      cell: ({ row }) => <IssueBadge row={row.original} />,
    }),
    columnHelper.display({
      id: 'menu',
      header: '',
      enableSorting: false,
      meta: { width: MENU_COLUMN_WIDTH },
      cell: ({ row }) => <RowMenu row={row.original} onManage={onManage} />,
    }),
  ])
}
