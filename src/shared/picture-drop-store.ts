import { create } from 'zustand'
import type { PictureDropState } from './types'

// A dropped image goes to whatever is open and can take one: a mod's details or a skin set. The
// window owns drops, those belong to their features; they meet here.
export const usePictureDropStore = create<PictureDropState>((set) => ({
  target: null,
  setTarget: (target) => set({ target }),
  clearTarget: () => set({ target: null }),
}))
