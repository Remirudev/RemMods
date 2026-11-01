import { create } from 'zustand'
import { EMPTY_FILTER } from './filter-mods'
import type { ModsUiState } from './types'

// UI state only; the rows themselves live in the query cache.
export const useModsUiStore = create<ModsUiState>((set) => ({
  filter: EMPTY_FILTER,
  tab: 'mods',
  detailsRef: null,
  activeIndex: -1,
  setText: (text) => set((state) => ({ filter: { ...state.filter, text } })),
  setStateFilter: (stateFilter) =>
    set((state) => ({ filter: { ...state.filter, state: stateFilter } })),
  setKinds: (kinds) => set((state) => ({ filter: { ...state.filter, kinds } })),
  setConflictsOnly: (conflictsOnly) =>
    set((state) => ({ filter: { ...state.filter, conflictsOnly } })),
  setIssuesOnly: (issuesOnly) => set((state) => ({ filter: { ...state.filter, issuesOnly } })),
  clearFilters: () => set({ filter: EMPTY_FILTER }),
  toggleConflictsView: () =>
    set((state) => {
      const showing = state.tab === 'mods' && state.filter.conflictsOnly
      return { tab: 'mods', filter: { ...state.filter, conflictsOnly: !showing } }
    }),
  setTab: (tab) => set({ tab }),
  openDetails: (modRef) => set({ detailsRef: modRef }),
  closeDetails: () => set({ detailsRef: null }),
  setActiveIndex: (activeIndex) => set({ activeIndex }),
}))
