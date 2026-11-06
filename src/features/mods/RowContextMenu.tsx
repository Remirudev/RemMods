import { Menu } from '@mantine/core'
import { cloneElement, type MouseEvent, useState } from 'react'
import type { ModRow } from '../../lib/ipc/types'
import { RowMenuItems } from './RowMenuItems'
import type { RowContextMenuProps } from './types'

// A right-click on a row opens its menu at the pointer. One Menu for the whole list: one per row
// took a tenth off the table's scroll speed, and scrolling would drop it from the virtual list.
export function RowContextMenu({ rows, onManage, children }: RowContextMenuProps) {
  const [row, setRow] = useState<ModRow | null>(null)

  const onContextMenu = (event: MouseEvent) => {
    const element = event.target instanceof Element ? event.target.closest('[data-mod-ref]') : null
    const ref = element?.getAttribute('data-mod-ref')
    const found = rows.find((candidate) => candidate.modRef === ref) ?? null
    // Anywhere else in the list (a column header, the gap between cards) no menu opens.
    if (found === null) event.preventDefault()
    setRow(found)
  }

  return (
    <Menu position="bottom-start" withinPortal>
      <Menu.ContextMenu>{cloneElement(children, { onContextMenu })}</Menu.ContextMenu>
      {row && <RowMenuItems row={row} onManage={onManage} />}
    </Menu>
  )
}
