import { create } from 'zustand'
import { pruneSelection } from './selection'
import type { GroupChoice, GroupFilterState } from './types'

// The groups sidebar lists the groups here; it and the list's Groups filter pick which of them the
// Mods list shows. Neither feature imports the other, both meet here.
export const useGroupFilterStore = create<GroupFilterState>((set) => ({
  groups: [],
  picked: [],
  anchor: null,
  // A group that is gone leaves the picks.
  setGroups: (groups) =>
    set((state) => {
      const ids = groups.map((group) => group.id)
      const anchor = state.anchor !== null && ids.includes(state.anchor) ? state.anchor : null
      return { groups, picked: pruneSelection(state.picked, ids), anchor }
    }),
  pickGroups: (picked, anchor) => set({ picked, anchor }),
  clearPicks: () => set({ picked: [], anchor: null }),
}))

// What the picked groups switch on, together; null while no group is picked.
export function pickedKeys(
  groups: readonly GroupChoice[],
  picked: readonly string[],
): string[] | null {
  if (picked.length === 0) return null
  const keys = groups.filter((group) => picked.includes(group.id)).flatMap((group) => group.keys)
  return [...new Set(keys)]
}
