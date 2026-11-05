import { create } from 'zustand'
import type { GroupPrefsState } from './types'

const ASK_KEY = 'rmods.groups.askBeforeSwitch'
const COLLAPSED_KEY = 'rmods.groups.collapsed'

// Storage can be missing or throw (private mode, cleared site data); both choices are only
// conveniences, so failures fall back to the defaults silently.
function readFlag(key: string, fallback: boolean): boolean {
  try {
    const stored = window.localStorage.getItem(key)
    return stored === null ? fallback : stored === 'true'
  } catch {
    return fallback
  }
}

function storeFlag(key: string, value: boolean): void {
  try {
    window.localStorage.setItem(key, String(value))
  } catch {
    // Not remembered for the next start; nothing else depends on it.
  }
}

export const useGroupPrefs = create<GroupPrefsState>((set) => ({
  askBeforeSwitch: readFlag(ASK_KEY, true),
  collapsed: readFlag(COLLAPSED_KEY, false),
  setAskBeforeSwitch: (askBeforeSwitch) => {
    storeFlag(ASK_KEY, askBeforeSwitch)
    set({ askBeforeSwitch })
  },
  setCollapsed: (collapsed) => {
    storeFlag(COLLAPSED_KEY, collapsed)
    set({ collapsed })
  },
}))
