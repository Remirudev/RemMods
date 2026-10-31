import { Box } from '@mantine/core'
import type { MouseEvent } from 'react'
import { runWindowAction } from './current-window'
import classes from './title-bar.module.css'
import type { ResizeEdge } from './types'
import { useWindowMaximized } from './use-window-maximized'

const RESIZE_EDGES: readonly ResizeEdge[] = [
  'North',
  'South',
  'West',
  'East',
  'NorthWest',
  'NorthEast',
  'SouthWest',
  'SouthEast',
]

function startResize(event: MouseEvent, edge: ResizeEdge): void {
  if (event.button !== 0) return
  event.preventDefault()
  runWindowAction((appWindow) => appWindow.startResizeDragging(edge))
}

export function ResizeHandles() {
  const maximized = useWindowMaximized()
  if (maximized) return null

  return RESIZE_EDGES.map((edge) => (
    <Box
      key={edge}
      aria-hidden
      className={classes.resizeHandle}
      data-edge={edge}
      onMouseDown={(event) => startResize(event, edge)}
    />
  ))
}
