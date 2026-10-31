import { create } from 'zustand'
import { DEFAULT_PANEL, isPanelId } from './panels'
import type { ActivePanelState, PanelId } from './types'

const STORAGE_KEY = 'rmods.activePanel'

// Storage can be missing or throw (private mode, cleared site data); the panel choice is only a
// convenience, so failures fall back silently.
function readStoredPanel(): PanelId {
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY)
    return isPanelId(stored) ? stored : DEFAULT_PANEL
  } catch {
    return DEFAULT_PANEL
  }
}

function storePanel(panel: PanelId): void {
  try {
    window.localStorage.setItem(STORAGE_KEY, panel)
  } catch {
    // Not remembered for the next start; nothing else depends on it.
  }
}

const initialPanel = readStoredPanel()

export const useActivePanelStore = create<ActivePanelState>((set) => ({
  activePanel: initialPanel,
  visited: [initialPanel],
  setActivePanel: (panel) => {
    storePanel(panel)
    set((state) => ({
      activePanel: panel,
      visited: state.visited.includes(panel) ? state.visited : [...state.visited, panel],
    }))
  },
}))
