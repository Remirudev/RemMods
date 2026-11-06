import { Table } from '@mantine/core'
import { useTable } from '@tanstack/react-table'
import { useVirtualizer } from '@tanstack/react-virtual'
import { useEffect, useMemo, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import type { ModRow } from '../../lib/ipc/types'
import { useHidePictures } from '../../shared/use-hide-pictures'
import { useColumnLayoutStore } from './column-layout-store'
import { NAME_SORTING } from './compare-mods'
import { ModHeaderCell } from './ModHeaderCell'
import { modTableFeatures } from './mod-table-features'
import { buildModColumns } from './mod-table-model'
import classes from './mods.module.css'
import { useModsUiStore } from './mods-ui-store'
import { RowContextMenu } from './RowContextMenu'
import type { ModTableProps } from './types'
import { useModListControls } from './use-mod-list-controls'

const ROW_HEIGHT = 48
const HEADER_HEIGHT = 40

// Rows outside the window are one empty row each side, so the scroll height stays right.
function spacerRow(key: string, height: number, columns: number) {
  return (
    <tr key={key}>
      <td aria-hidden="true" colSpan={columns} style={{ height, padding: 0, border: 0 }} />
    </tr>
  )
}

function getRowId(row: ModRow): string {
  return row.modRef
}

export function ModTable({ rows, onOpen, onToggle, onManage, onBulk }: ModTableProps) {
  const { t, i18n } = useTranslation('mods')
  const locale = i18n.language
  const columns = useMemo(
    () => buildModColumns(t, locale, onToggle, onManage),
    [t, locale, onToggle, onManage],
  )
  const sorting = useModsUiStore((state) => state.sorting)
  const setSorting = useModsUiStore((state) => state.setSorting)
  const columnSizing = useColumnLayoutStore((state) => state.sizing)
  const storedVisibility = useColumnLayoutStore((state) => state.visibility)
  const hidePictures = useHidePictures()
  const columnVisibility = useMemo(
    () => (hidePictures ? { ...storedVisibility, picture: false } : storedVisibility),
    [hidePictures, storedVisibility],
  )
  const viewportRef = useRef<HTMLDivElement>(null)

  const table = useTable({
    features: modTableFeatures,
    columns,
    data: rows as ModRow[],
    getRowId,
    state: { sorting, columnSizing, columnVisibility },
    onSortingChange: setSorting,
    enableSortingRemoval: false,
    enableMultiSort: false,
  })
  const sortedRows = table.getRowModel().rows
  // Without a stored width Name takes the free space, so the table spans the pane; once it has
  // one, every column is exact and the table is as wide as their sum (scrolling sideways if wider).
  const nameFills = columnSizing.name === undefined
  const totalWidth = table.getTotalSize()

  // The collator changes with the language, so the same sort state has to run again.
  // biome-ignore lint/correctness/useExhaustiveDependencies: `locale` is the trigger, not a read value.
  useEffect(() => {
    setSorting((current) => [...current])
  }, [locale, setSorting])

  // Rows sorted by a column that is no longer shown would look unsorted, so Name takes over.
  const sortedBy = sorting[0]?.id
  useEffect(() => {
    if (sortedBy !== undefined && columnVisibility[sortedBy] === false) setSorting(NAME_SORTING)
  }, [sortedBy, columnVisibility, setSorting])

  const virtualizer = useVirtualizer({
    count: sortedRows.length,
    getScrollElement: () => viewportRef.current,
    estimateSize: () => ROW_HEIGHT,
    getItemKey: (index) => sortedRows[index]?.id ?? index,
    overscan: 10,
    scrollPaddingEnd: HEADER_HEIGHT,
  })
  const items = virtualizer.getVirtualItems()
  const first = items[0]
  const last = items[items.length - 1]
  const topSpace = first?.start ?? 0
  const bottomSpace = last ? virtualizer.getTotalSize() - last.end : 0
  const columnCount = table.getVisibleLeafColumns().length
  const sorted = useMemo(() => sortedRows.map((row) => row.original), [sortedRows])
  const { activeIndex, selected, onItemClick, onKeyDown } = useModListControls({
    rows,
    sorted,
    columns: 1,
    pageSize: () => Math.floor((viewportRef.current?.clientHeight ?? 0) / ROW_HEIGHT) - 1,
    scrollTo: (index) => virtualizer.scrollToIndex(index),
    onOpen,
    onToggle,
    onManage,
    onBulk,
  })
  const activeRendered = items.some((item) => item.index === activeIndex)

  return (
    <div className={classes.tableWrapper}>
      <Table.ScrollContainer
        minWidth={totalWidth}
        h="100%"
        className={classes.tableContainer}
        scrollAreaProps={{ viewportRef }}
      >
        <RowContextMenu rows={rows} onManage={onManage}>
          <Table
            stickyHeader
            layout="fixed"
            highlightOnHover
            className={classes.table}
            style={nameFills ? undefined : { width: totalWidth }}
            role="grid"
            tabIndex={0}
            aria-multiselectable="true"
            aria-rowcount={sortedRows.length + 1}
            aria-activedescendant={activeRendered ? `mods-row-${activeIndex}` : undefined}
            aria-label={t(($) => $.table.label)}
            onKeyDown={onKeyDown}
          >
            <Table.Thead>
              {table.getHeaderGroups().map((group) => (
                <Table.Tr key={group.id}>
                  {group.headers.map((header) => (
                    <ModHeaderCell
                      key={header.id}
                      header={header}
                      fills={nameFills && header.column.id === 'name'}
                    />
                  ))}
                </Table.Tr>
              ))}
            </Table.Thead>
            <Table.Tbody>
              {topSpace > 0 && spacerRow('top', topSpace, columnCount)}
              {items.map((item) => {
                const row = sortedRows[item.index]
                if (!row) return null
                return (
                  <Table.Tr
                    key={row.id}
                    id={`mods-row-${item.index}`}
                    data-testid="mods-row"
                    data-mod-ref={row.id}
                    className={classes.row}
                    aria-rowindex={item.index + 2}
                    aria-selected={selected.has(row.id)}
                    mod={{ active: item.index === activeIndex, selected: selected.has(row.id) }}
                    onClick={(event) => onItemClick(item.index, event)}
                  >
                    {row.getVisibleCells().map((cell) => (
                      <Table.Td
                        key={cell.id}
                        ta={cell.column.columnDef.meta?.align === 'right' ? 'right' : undefined}
                      >
                        <table.FlexRender cell={cell} />
                      </Table.Td>
                    ))}
                  </Table.Tr>
                )
              })}
              {bottomSpace > 0 && spacerRow('bottom', bottomSpace, columnCount)}
            </Table.Tbody>
          </Table>
        </RowContextMenu>
      </Table.ScrollContainer>
    </div>
  )
}
