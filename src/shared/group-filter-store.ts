import { create } from 'zustand'
import type { GroupFilterState } from './types'

// The groups sidebar sets the group whose mods the Mods list shows; neither feature imports the
// other, both meet here.
export const useGroupFilterStore = create<GroupFilterState>((set) => ({
  group: null,
  showGroup: (group) => set({ group }),
  clearGroup: () => set({ group: null }),
}))
