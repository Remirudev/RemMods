import type { SkinSet } from '../../lib/ipc/types'

export interface SkinSetRowProps {
  set: SkinSet
  active: boolean
  disabled: boolean
}

export interface SkinSetDialogProps {
  setId: string
  modalId: string
  onClose: () => void
}

export interface SkinSetEditorProps {
  set: SkinSet
  active: boolean
  locked: boolean
  modalId: string
  onClose: () => void
}

export interface NewSkinSetDialogProps {
  modalId: string
  copy?: string
  onClose: () => void
}

export interface NewSkinSet {
  name: string
  copy?: string | undefined
}

export interface ScreenshotListProps {
  set: SkinSet
}

export interface InterruptedSwitchProps {
  setId: string
  name: string
}
