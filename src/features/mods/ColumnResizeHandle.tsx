import { type KeyboardEvent, type MouseEvent, useEffect, useRef, useState } from 'react'
import { keyboardWidth } from './column-layout'
import classes from './mods.module.css'
import type { ColumnResizeHandleProps } from './types'

// Styled in mods.module.css: the resize cursor everywhere while a drag runs.
const RESIZING_ATTRIBUTE = 'data-column-resizing'

function clamp(width: number, min: number, max: number): number {
  return Math.round(Math.min(max, Math.max(min, width)))
}

// The right edge of a header cell: drag it, or focus it and use the arrow keys (Shift for bigger
// steps); a double click goes back to the default width. Widths are saved once per drag.
export function ColumnResizeHandle({
  label,
  width,
  minWidth,
  maxWidth,
  measure,
  onResize,
  onCommit,
  onReset,
}: ColumnResizeHandleProps) {
  const [dragging, setDragging] = useState(false)
  const stopDrag = useRef<(() => void) | null>(null)

  // A header that goes away mid-drag must not leave its listeners on the document.
  useEffect(() => () => stopDrag.current?.(), [])

  const onMouseDown = (event: MouseEvent<HTMLDivElement>) => {
    if (event.button !== 0) return
    event.preventDefault()
    stopDrag.current?.()
    const startX = event.clientX
    const start = measure()
    const root = document.documentElement
    let moved = false
    const onMove = (move: globalThis.MouseEvent) => {
      const next = clamp(start + move.clientX - startX, minWidth, maxWidth)
      if (!moved && next === start) return
      moved = true
      onResize(next)
    }
    const stop = () => {
      document.removeEventListener('mousemove', onMove)
      document.removeEventListener('mouseup', stop)
      root.removeAttribute(RESIZING_ATTRIBUTE)
      stopDrag.current = null
      setDragging(false)
      if (moved) onCommit()
    }
    document.addEventListener('mousemove', onMove)
    document.addEventListener('mouseup', stop)
    root.setAttribute(RESIZING_ATTRIBUTE, '')
    stopDrag.current = stop
    setDragging(true)
  }

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const next = keyboardWidth(measure(), event.key, event.shiftKey)
    if (next === null) return
    event.preventDefault()
    onResize(clamp(next, minWidth, maxWidth))
    onCommit()
  }

  return (
    // biome-ignore lint/a11y/useSemanticElements: an <hr> takes no focus or drag; a focusable separator is the ARIA pattern for a resize handle.
    <div
      role="separator"
      aria-orientation="vertical"
      aria-label={label}
      aria-valuenow={Math.round(width)}
      aria-valuemin={minWidth}
      aria-valuemax={maxWidth}
      tabIndex={0}
      className={classes.resizeHandle}
      data-dragging={dragging || undefined}
      onMouseDown={onMouseDown}
      onKeyDown={onKeyDown}
      onDoubleClick={onReset}
    />
  )
}
