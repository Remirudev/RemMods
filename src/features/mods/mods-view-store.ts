import { create } from 'zustand'
import type { ModsView, ModsViewState } from './types'

const VIEW_KEY = 'rmods.modsView.v1'

// Like the column layout: storage can be missing or throw, and the view is only a convenience.
function readStoredView(): ModsView {
  try {
    return window.localStorage.getItem(VIEW_KEY) === 'grid' ? 'grid' : 'table'
  } catch {
    return 'table'
  }
}

function storeView(view: ModsView): void {
  try {
    window.localStorage.setItem(VIEW_KEY, view)
  } catch {
    // Not remembered for the next start; this session keeps the view.
  }
}

export const useModsViewStore = create<ModsViewState>((set) => ({
  view: readStoredView(),
  setView: (view) => {
    set({ view })
    storeView(view)
  },
}))
