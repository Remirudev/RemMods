import { useSkinSets } from '../../shared/skin-sets'
import { SkinSetEditor } from './SkinSetEditor'
import type { SkinSetDialogProps } from './types'

// The form starts from the stored set, so it waits for the list; a set deleted meanwhile is gone.
export function SkinSetDialog({ setId, modalId, onClose }: SkinSetDialogProps) {
  const view = useSkinSets().data
  const set = view?.sets.find((candidate) => candidate.id === setId)
  if (!view || !set) return null

  return (
    <SkinSetEditor
      set={set}
      active={view.active === set.id}
      locked={view.readOnly || view.interrupted !== null}
      modalId={modalId}
      onClose={onClose}
    />
  )
}
