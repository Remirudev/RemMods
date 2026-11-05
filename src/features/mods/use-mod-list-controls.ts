import { type KeyboardEvent, type MouseEvent, useEffect, useMemo, useRef } from 'react'
import { canManage, rowSubject } from './manage-model'
import { bulkCount, spaceAction } from './mod-actions'
import { useModsUiStore } from './mods-ui-store'
import { applyClick, extendTo } from './selection'
import { nextActiveIndex, tableKeyIntent } from './table-keyboard'
import type { ModListControls, ModListControlsInput } from './types'

// Selection, the active item and the keyboard, the same for the table and the grid.
export function useModListControls({
  rows,
  sorted,
  columns,
  pageSize,
  scrollTo,
  onOpen,
  onToggle,
  onManage,
  onBulk,
}: ModListControlsInput): ModListControls {
  const activeIndex = useModsUiStore((state) => state.activeIndex)
  const setActiveIndex = useModsUiStore((state) => state.setActiveIndex)
  const selection = useModsUiStore((state) => state.selection)
  const anchor = useModsUiStore((state) => state.anchor)
  const setSelection = useModsUiStore((state) => state.setSelection)
  const clearSelection = useModsUiStore((state) => state.clearSelection)
  const selected = useMemo(() => new Set(selection), [selection])
  const activeMemory = useRef<{ modRef: string; key: string } | null>(null)
  const ids = useMemo(() => sorted.map((row) => row.modRef), [sorted])

  // A LogicMods toggle renames the folder, so the active row comes back under a new ref: follow
  // it by key instead of leaving the cursor on whatever row moved into its place.
  useEffect(() => {
    const remembered = activeMemory.current
    const current = sorted[activeIndex]
    activeMemory.current = current ? { modRef: current.modRef, key: current.key } : null
    if (!remembered || sorted.some((row) => row.modRef === remembered.modRef)) return
    const moved = sorted.findIndex((row) => row.key === remembered.key)
    if (moved >= 0) setActiveIndex(moved)
  }, [sorted, activeIndex, setActiveIndex])

  const openItem = (index: number) => {
    const row = sorted[index]
    if (!row) return
    setActiveIndex(index)
    onOpen(row.modRef)
  }

  const onItemClick = (index: number, event: MouseEvent) => {
    const row = sorted[index]
    if (!row) return
    const shift = event.shiftKey
    const ctrl = event.ctrlKey || event.metaKey
    const next = applyClick(selection, anchor, ids, row.modRef, { shift, ctrl })
    setSelection(next.selection, next.anchor)
    setActiveIndex(index)
    if (!shift && !ctrl) onOpen(row.modRef)
  }

  const activeRef = () => {
    if (activeIndex < 0 || sorted.length === 0) return null
    return sorted[Math.min(activeIndex, sorted.length - 1)]?.modRef ?? null
  }

  const toggleActive = () => {
    const action = spaceAction(rows, selection, activeRef())
    if (action) onToggle(action.rows, action.enable)
  }

  // Like Space: with the active row among two or more selected rows, Delete and Shift+Delete act
  // on the selection (gated on what the selection allows, not on the active row).
  const manageActive = (action: 'rename' | 'trash' | 'delete') => {
    const id = activeRef()
    if (action !== 'rename' && id !== null && selection.length >= 2 && selection.includes(id)) {
      if (bulkCount(rows, selection, action) > 0) onBulk(action)
      return
    }
    const row = sorted.find((candidate) => candidate.modRef === id)
    if (row && canManage(rowSubject(row), action)) onManage(action, row)
  }

  const moveActive = (key: string, extend: boolean) => {
    const next = nextActiveIndex(activeIndex, key, sorted.length, pageSize(), columns)
    if (next === null) return false
    setActiveIndex(next)
    scrollTo(next)
    const target = sorted[next]
    if (extend && target) {
      const range = extendTo(selection, anchor ?? activeRef(), ids, target.modRef)
      setSelection(range.selection, range.anchor)
    }
    return true
  }

  const onKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (event.target !== event.currentTarget) return
    const intent = tableKeyIntent(event.key, event.shiftKey)
    if (!intent) return
    switch (intent.kind) {
      case 'open':
        if (activeIndex < 0) return
        event.preventDefault()
        openItem(Math.min(activeIndex, sorted.length - 1))
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

  return { activeIndex, selected, onItemClick, onKeyDown }
}
