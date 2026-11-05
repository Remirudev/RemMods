import { create } from 'zustand'
import type { UpdateRunState } from './types'

// UI state only: set while a self-update downloads and installs. Nothing may start other work or
// leave the Settings panel then, so the progress dialog and any error stay in view.
export const useUpdateRun = create<UpdateRunState>((set) => ({
  updating: false,
  setUpdating: (updating) => set({ updating }),
}))

export function isUpdating(): boolean {
  return useUpdateRun.getState().updating
}
