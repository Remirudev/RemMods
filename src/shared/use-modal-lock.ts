import { modals } from '@mantine/modals'
import { useEffect } from 'react'

// While a dialog's file change runs (or waits for a folder in use), Escape, a click outside and
// the close button stay off: closing it would look like a cancel that did not happen. The
// folder-busy toast's Cancel is the one way to stop a wait.
export function useModalLock(modalId: string | undefined, locked: boolean): void {
  useEffect(() => {
    if (modalId === undefined) return
    modals.updateModal({
      modalId,
      closeOnEscape: !locked,
      closeOnClickOutside: !locked,
      withCloseButton: !locked,
    })
  }, [modalId, locked])
}
