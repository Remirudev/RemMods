import { create } from 'zustand'
import type { PictureDropState } from './types'

// A dropped image becomes the picture of the mod whose details are open. The window owns drops,
// the details belong to the mods feature; they meet here.
export const usePictureDropStore = create<PictureDropState>((set) => ({
  target: null,
  name: null,
  setTarget: (target, name) => set({ target, name }),
  clearTarget: () => set({ target: null, name: null }),
}))
