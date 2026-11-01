import { Table, UnstyledButton } from '@mantine/core'
import { IconArrowDown, IconArrowsSort, IconArrowUp } from '@tabler/icons-react'
import { type SortingState, useTable } from '@tanstack/react-table'
import { useVirtualizer } from '@tanstack/react-virtual'
import { type KeyboardEvent, type MouseEvent, useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { ModRow } from '../../lib/ipc/types'
import { canManage, rowSubject } from './manage-model'
import { spaceAction } from './mod-actions'
import { buildModColumns, modTableFeatures } from './mod-table-model'
import classes from './mods.module.css'
import { useModsUiStore } from './mods-ui-store'
import { applyClick, extendTo } from './selection'
import { nextActiveIndex, tableKeyIntent } from './table-keyboard'
import type { ModTableProps } from './types'

const ROW_HEIGHT = 40
const HEADER_HEIGHT = 40
const INITIAL_SORTING: SortingState = [{ id: 'name', desc: false }]

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

export function ModTable({ rows, onOpen, onToggle, onManage }: ModTableProps) {
  const { t, i18n } = useTranslation('mods')
  const locale = i18n.language
  const columns = useMemo(
    () => buildModColumns(t, locale, onToggle, onManage),
    [t, locale, onToggle, onManage],
  )
  const [sorting, setSorting] = useState<SortingState>(INITIAL_SORTING)
  const viewportRef = useRef<HTMLDivElement>(null)
  const activeIndex = useModsUiStore((state) => state.activeIndex)
  const setActiveIndex = useModsUiStore((state) => state.setActiveIndex)
  const selection = useModsUiStore((state) => state.selection)
  const anchor = useModsUiStore((state) => state.anchor)
  const setSelection = useModsUiStore((state) => state.setSelection)
  const clearSelection = useModsUiStore((state) => state.clearSelection)
  const selected = useMemo(() => new Set(selection), [selection])
  const activeMemory = useRef<{ modRef: string; key: string } | null>(null)

  const table = useTable({
    features: modTableFeatures,
    columns,
    data: rows as ModRow[],
    getRowId,
    state: { sorting },
    onSortingChange: setSorting,
    enableSortingRemoval: false,
    enableMultiSort: false,
  })
  const sortedRows = table.getRowModel().rows

  // The collator changes with the language, so the same sort state has to run again.
  // biome-ignore lint/correctness/useExhaustiveDependencies: `locale` is the trigger, not a read value.
  useEffect(() => {
    setSorting((current) => [...current])
  }, [locale])

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
  const columnCount = columns.length
  const activeRendered = items.some((item) => item.index === activeIndex)
  const ids = useMemo(() => sortedRows.map((row) => row.id), [sortedRows])

  // A LogicMods toggle renames the folder, so the active row comes back under a new ref: follow
  // it by key instead of leaving the cursor on whatever row moved into its place.
  useEffect(() => {
    const remembered = activeMemory.current
    const current = sortedRows[activeIndex]?.original
    activeMemory.current = current ? { modRef: current.modRef, key: current.key } : null
    if (!remembered || sortedRows.some((row) => row.id === remembered.modRef)) return
    const moved = sortedRows.findIndex((row) => row.original.key === remembered.key)
    if (moved >= 0) setActiveIndex(moved)
  }, [sortedRows, activeIndex, setActiveIndex])

  const openRow = (index: number) => {
    const row = sortedRows[index]
    if (!row) return
    setActiveIndex(index)
    onOpen(row.original.modRef)
  }

  const onRowClick = (index: number, event: MouseEvent) => {
    const row = sortedRows[index]
    if (!row) return
    const shift = event.shiftKey
    const ctrl = event.ctrlKey || event.metaKey
    const next = applyClick(selection, anchor, ids, row.id, { shift, ctrl })
    setSelection(next.selection, next.anchor)
    setActiveIndex(index)
    if (!shift && !ctrl) onOpen(row.original.modRef)
  }

  const activeRowId = () => {
    if (activeIndex < 0 || sortedRows.length === 0) return null
    return sortedRows[Math.min(activeIndex, sortedRows.length - 1)]?.id ?? null
  }

  const toggleActive = () => {
    const action = spaceAction(rows, selection, activeRowId())
    if (action) onToggle(action.rows, action.enable)
  }

  const manageActive = (action: 'rename' | 'trash' | 'delete') => {
    const id = activeRowId()
    const row = sortedRows.find((candidate) => candidate.id === id)?.original
    if (row && canManage(rowSubject(row), action)) onManage(action, row)
  }

  const moveActive = (key: string, extend: boolean) => {
    const pageSize = Math.floor((viewportRef.current?.clientHeight ?? 0) / ROW_HEIGHT) - 1
    const next = nextActiveIndex(activeIndex, key, sortedRows.length, pageSize)
    if (next === null) return false
    setActiveIndex(next)
    virtualizer.scrollToIndex(next)
    const target = sortedRows[next]
    if (extend && target) {
      const range = extendTo(selection, anchor ?? activeRowId(), ids, target.id)
      setSelection(range.selection, range.anchor)
    }
    return true
  }

  const onKeyDown = (event: KeyboardEvent<HTMLTableElement>) => {
    if (event.target !== event.currentTarget) return
    const intent = tableKeyIntent(event.key, event.shiftKey)
    if (!intent) return
    switch (intent.kind) {
      case 'open':
        if (activeIndex < 0) return
        event.preventDefault()
        openRow(Math.min(activeIndex, sortedRows.length - 1))
        return
      case 'space':
        event.preventDefault()
        toggleActive()
        return
      case 'manage':
        event.preventDefault()
        manageActive(intent.action)
        return
      case 'escape':
        if (selection.length === 0) return
        event.preventDefault()
        clearSelection()
        return
      case 'move':
      case 'extend':
        if (moveActive(intent.key, intent.kind === 'extend')) event.preventDefault()
        return
    }
  }

  return (
    <div className={classes.tableWrapper}>
      <Table.ScrollContainer
        minWidth={900}
        h="100%"
        className={classes.tableContainer}
        scrollAreaProps={{ viewportRef }}
      >
        <Table
          stickyHeader
          layout="fixed"
          highlightOnHover
          className={classes.table}
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
                {group.headers.map((header) => {
                  const sorted = header.column.getIsSorted()
                  const meta = header.column.columnDef.meta
                  return (
                    <Table.Th
                      key={header.id}
                      className={classes.headerCell}
                      style={{ width: meta?.width }}
                      aria-sort={
                        sorted === 'asc' ? 'ascending' : sorted === 'desc' ? 'descending' : 'none'
                      }
                      ta={meta?.align === 'right' ? 'right' : undefined}
                    >
                      {header.column.getCanSort() ? (
                        <UnstyledButton
                          className={classes.sortButton}
                          onClick={header.column.getToggleSortingHandler()}
                        >
                          <table.FlexRender header={header} />
                          {sorted === 'asc' ? (
                            <IconArrowUp size={14} stroke={1.8} />
                          ) : sorted === 'desc' ? (
                            <IconArrowDown size={14} stroke={1.8} />
                          ) : (
                            <IconArrowsSort size={14} stroke={1.8} opacity={0.5} />
                          )}
                        </UnstyledButton>
                      ) : (
                        <table.FlexRender header={header} />
                      )}
                    </Table.Th>
                  )
                })}
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
                  className={classes.row}
                  aria-rowindex={item.index + 2}
                  aria-selected={selected.has(row.id)}
                  mod={{ active: item.index === activeIndex, selected: selected.has(row.id) }}
                  onClick={(event) => onRowClick(item.index, event)}
                >
                  {row.getAllCells().map((cell) => (
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
      </Table.ScrollContainer>
    </div>
  )
}
