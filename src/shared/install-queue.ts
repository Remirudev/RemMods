import { create } from 'zustand'
import type { InstallQueueState } from './types'

// UI state only: sources waiting to be installed one after the other. The paths stay in Rust;
// the queue holds tokens and archive row refs.
export const useInstallQueue = create<InstallQueueState>((set) => ({
  queue: [],
  current: null,
  turn: 0,
  push: (sources) =>
    set((state) => {
      if (sources.length === 0) return state
      const queue = [...state.queue, ...sources]
      return {
        queue,
        current: queue[0] ?? null,
        turn: state.queue.length === 0 ? state.turn + 1 : state.turn,
      }
    }),
  shift: () =>
    set((state) => {
      const queue = state.queue.slice(1)
      return { queue, current: queue[0] ?? null, turn: state.turn + 1 }
    }),
  clear: () => set((state) => ({ queue: [], current: null, turn: state.turn + 1 })),
}))
