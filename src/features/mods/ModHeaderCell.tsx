import { Table, UnstyledButton } from '@mantine/core'
import { IconArrowDown, IconArrowsSort, IconArrowUp } from '@tabler/icons-react'
import { FlexRender } from '@tanstack/react-table'
import { useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { ColumnResizeHandle } from './ColumnResizeHandle'
import { isModColumnId } from './column-layout'
import { useColumnLayoutStore } from './column-layout-store'
import classes from './mods.module.css'
import type { ModHeaderCellProps } from './types'

export function ModHeaderCell({ header, fills }: ModHeaderCellProps) {
  const { t } = useTranslation('mods')
  const cellRef = useRef<HTMLTableCellElement>(null)
  const setWidth = useColumnLayoutStore((state) => state.setWidth)
  const saveLayout = useColumnLayoutStore((state) => state.saveLayout)
  const resetWidth = useColumnLayoutStore((state) => state.resetWidth)
  const { column } = header
  const sorted = column.getIsSorted()
  const meta = column.columnDef.meta
  const title = typeof column.columnDef.header === 'string' ? column.columnDef.header : ''
  const size = header.getSize()
  const resizable = meta?.resizable === true && isModColumnId(column.id) ? column.id : null

  // A filling Name column is often drawn wider than its default; resizing starts from what is seen.
  const measure = () => {
    if (!fills) return size
    const drawn = cellRef.current?.getBoundingClientRect().width ?? 0
    return Math.max(size, Math.round(drawn))
  }

  return (
    <Table.Th
      ref={cellRef}
      className={classes.headerCell}
      style={{ width: fills ? undefined : size }}
      aria-label={title === '' ? undefined : title}
      aria-sort={sorted === 'asc' ? 'ascending' : sorted === 'desc' ? 'descending' : 'none'}
      ta={meta?.align === 'right' ? 'right' : undefined}
    >
      {column.getCanSort() ? (
        <UnstyledButton className={classes.sortButton} onClick={column.getToggleSortingHandler()}>
          <span className={classes.headerLabel}>
            <FlexRender header={header} />
          </span>
          {sorted === 'asc' ? (
            <IconArrowUp size={14} stroke={1.8} className={classes.sortIcon} />
          ) : sorted === 'desc' ? (
            <IconArrowDown size={14} stroke={1.8} className={classes.sortIcon} />
          ) : (
            <IconArrowsSort size={14} stroke={1.8} opacity={0.5} className={classes.sortIcon} />
          )}
        </UnstyledButton>
      ) : (
        <span className={classes.headerLabel}>
          <FlexRender header={header} />
        </span>
      )}
      {resizable !== null && (
        <ColumnResizeHandle
          label={t(($) => $.table.resizeColumn, { column: title })}
          width={size}
          minWidth={column.columnDef.minSize ?? size}
          maxWidth={column.columnDef.maxSize ?? size}
          measure={measure}
          onResize={(width) => setWidth(resizable, width)}
          onCommit={saveLayout}
          onReset={() => resetWidth(resizable)}
        />
      )}
    </Table.Th>
  )
}
