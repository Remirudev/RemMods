import { create } from 'zustand'
import { useGroupFilterStore } from '../../shared/group-filter-store'
import { EMPTY_FILTER } from './filter-mods'
import { movedRefs, repointRef } from './ref-repoint'
import type { ModsUiState } from './types'

// UI state only; the rows themselves live in the query cache.
export const useModsUiStore = create<ModsUiState>((set) => ({
  filter: EMPTY_FILTER,
  tab: 'mods',
  detailsRef: null,
  detailsCharacter: null,
  activeIndex: -1,
  selection: [],
  anchor: null,
  setText: (text) => set((state) => ({ filter: { ...state.filter, text } })),
  setStateFilter: (stateFilter) =>
    set((state) => ({ filter: { ...state.filter, state: stateFilter } })),
  setKinds: (kinds) => set((state) => ({ filter: { ...state.filter, kinds } })),
  setConflictsOnly: (conflictsOnly) =>
    set((state) => ({ filter: { ...state.filter, conflictsOnly } })),
  setContentConflictsOnly: (contentConflictsOnly) =>
    set((state) => ({ filter: { ...state.filter, contentConflictsOnly } })),
  setIssuesOnly: (issuesOnly) => set((state) => ({ filter: { ...state.filter, issuesOnly } })),
  // A group picked in the sidebar is a filter too, so "Clear filters" drops it as well.
  clearFilters: () => {
    useGroupFilterStore.getState().clearGroup()
    set({ filter: EMPTY_FILTER })
  },
  toggleConflictsView: () =>
    set((state) => {
      const showing = state.tab === 'mods' && state.filter.conflictsOnly
      return { tab: 'mods', filter: { ...state.filter, conflictsOnly: !showing } }
    }),
  setTab: (tab) => set({ tab }),
  openDetails: (modRef) => set({ detailsRef: modRef }),
  closeDetails: () => set({ detailsRef: null }),
  setDetailsCharacter: (detailsCharacter) => set({ detailsCharacter }),
  setActiveIndex: (activeIndex) => set({ activeIndex }),
  setSelection: (selection, anchor) => set({ selection, anchor }),
  clearSelection: () => set({ selection: [], anchor: null }),
  // Called with the rows of the previous and the new snapshot: refs that moved (a Logic folder
  // rename) are followed by key, refs of rows that are gone are dropped.
  reconcileRefs: (previous, next) =>
    set((state) => {
      const moved = previous ? movedRefs(previous, next) : new Map<string, string>()
      const follow = (ref: string) => repointRef(ref, moved, next)
      const selection = [
        ...new Set(state.selection.map(follow).filter((ref): ref is string => ref !== null)),
      ]
      return {
        selection,
        anchor: state.anchor === null ? null : follow(state.anchor),
        detailsRef: state.detailsRef === null ? null : follow(state.detailsRef),
      }
    }),
}))
