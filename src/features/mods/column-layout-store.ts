import { create } from 'zustand'
import {
  COLUMN_LAYOUT_KEY,
  clampColumnWidth,
  defaultColumnLayout,
  parseColumnLayout,
  serializeColumnLayout,
} from './column-layout'
import type { ColumnLayout, ColumnLayoutState } from './types'

// Like the active panel: storage can be missing or throw (private mode, cleared site data), and
// the layout is only a convenience, so failures fall back to the defaults silently.
function readStoredLayout(): ColumnLayout {
  try {
    return parseColumnLayout(window.localStorage.getItem(COLUMN_LAYOUT_KEY))
  } catch {
    return defaultColumnLayout()
  }
}

function storeLayout(layout: ColumnLayout): void {
  try {
    window.localStorage.setItem(COLUMN_LAYOUT_KEY, serializeColumnLayout(layout))
  } catch {
    // Not remembered for the next start; this session keeps the layout.
  }
}

function forgetLayout(): void {
  try {
    window.localStorage.removeItem(COLUMN_LAYOUT_KEY)
  } catch {
    // The next start may bring the old layout back; this session shows the defaults.
  }
}

// Widths and visible columns of the mod table, kept across starts in localStorage.
export const useColumnLayoutStore = create<ColumnLayoutState>((set, get) => ({
  ...readStoredLayout(),
  setWidth: (id, width) =>
    set((state) => ({ sizing: { ...state.sizing, [id]: clampColumnWidth(id, width) } })),
  saveLayout: () => storeLayout(get()),
  resetWidth: (id) => {
    set((state) => ({
      sizing: Object.fromEntries(Object.entries(state.sizing).filter(([column]) => column !== id)),
    }))
    storeLayout(get())
  },
  setColumnVisible: (id, visible) => {
    set((state) => ({ visibility: { ...state.visibility, [id]: visible } }))
    storeLayout(get())
  },
  resetLayout: () => {
    set(defaultColumnLayout())
    forgetLayout()
  },
}))
