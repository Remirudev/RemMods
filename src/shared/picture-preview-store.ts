import { create } from 'zustand'
import type { PicturePreviewState } from './types'

// One big picture at a time: a picture under the pointer asks, the app shows it.
export const usePicturePreviewStore = create<PicturePreviewState>((set) => ({
  preview: null,
  showPreview: (preview) => set({ preview }),
  hidePreview: () => set({ preview: null }),
}))
