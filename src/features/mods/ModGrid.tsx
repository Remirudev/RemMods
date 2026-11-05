import { ScrollArea, Table } from '@mantine/core'
import { useElementSize } from '@mantine/hooks'
import { useVirtualizer } from '@tanstack/react-virtual'
import { useMemo, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { sortMods } from './compare-mods'
import { ModCard } from './ModCard'
import classes from './mods.module.css'
import { useModsUiStore } from './mods-ui-store'
import type { ModTableProps } from './types'
import { useModListControls } from './use-mod-list-controls'

const CARD_MIN_WIDTH = 210
const CARD_HEIGHT = 300
const GAP = 12
const LINE_HEIGHT = CARD_HEIGHT + GAP

// Only the lines on screen exist, like the table's rows; the whole grid is one tab stop.
export function ModGrid({ rows, onOpen, onToggle, onManage, onBulk }: ModTableProps) {
  const { t, i18n } = useTranslation('mods')
  const sorting = useModsUiStore((state) => state.sorting)
  const sorted = useMemo(
    () => sortMods(rows, sorting, i18n.language),
    [rows, sorting, i18n.language],
  )
  const viewportRef = useRef<HTMLDivElement>(null)
  const { ref: gridRef, width } = useElementSize()
  const columns = Math.max(1, Math.floor((width + GAP) / (CARD_MIN_WIDTH + GAP)))
  const lines = Math.ceil(sorted.length / columns)
  const virtualizer = useVirtualizer({
    count: lines,
    getScrollElement: () => viewportRef.current,
    estimateSize: () => LINE_HEIGHT,
    overscan: 2,
  })
  const { activeIndex, selected, onItemClick, onKeyDown } = useModListControls({
    rows,
    sorted,
    columns,
    pageSize: () => Math.floor((viewportRef.current?.clientHeight ?? 0) / LINE_HEIGHT),
    scrollTo: (index) => virtualizer.scrollToIndex(Math.floor(index / columns)),
    onOpen,
    onToggle,
    onManage,
    onBulk,
  })
  const shown = virtualizer.getVirtualItems()
  const activeShown = shown.some((line) => line.index === Math.floor(activeIndex / columns))

  return (
    <div className={classes.tableWrapper}>
      <ScrollArea h="100%" className={classes.tableContainer} viewportRef={viewportRef}>
        <div ref={gridRef}>
          <Table
            withRowBorders={false}
            horizontalSpacing={0}
            verticalSpacing={0}
            className={classes.modGrid}
            style={{ height: virtualizer.getTotalSize() }}
            role="grid"
            tabIndex={0}
            aria-multiselectable="true"
            aria-rowcount={lines}
            aria-colcount={columns}
            aria-activedescendant={activeShown ? `mods-card-${activeIndex}` : undefined}
            aria-label={t(($) => $.table.label)}
            onKeyDown={onKeyDown}
          >
            <Table.Tbody>
              {shown.map((line) => (
                <Table.Tr
                  key={line.index}
                  aria-rowindex={line.index + 1}
                  className={classes.gridLine}
                  style={{
                    gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
                    gap: GAP,
                    height: CARD_HEIGHT,
                    transform: `translateY(${line.start}px)`,
                  }}
                >
                  {sorted
                    .slice(line.index * columns, (line.index + 1) * columns)
                    .map((row, offset) => {
                      const index = line.index * columns + offset
                      return (
                        <ModCard
                          key={row.modRef}
                          row={row}
                          index={index}
                          active={index === activeIndex}
                          selected={selected.has(row.modRef)}
                          onClick={onItemClick}
                          onToggle={onToggle}
                          onManage={onManage}
                        />
                      )
                    })}
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
        </div>
      </ScrollArea>
    </div>
  )
}
