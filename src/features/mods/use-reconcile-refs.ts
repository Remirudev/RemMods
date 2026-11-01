import { useLayoutEffect, useRef } from 'react'
import type { ModRow } from '../../lib/ipc/types'
import { useModsUiStore } from './mods-ui-store'

// Runs for every new snapshot before the passive effects of the children, so the drawer and the
// selection already point at the renamed rows when they look for them.
export function useReconcileRefs(rows: readonly ModRow[] | undefined): void {
  const previous = useRef<readonly ModRow[] | undefined>(undefined)
  const reconcileRefs = useModsUiStore((state) => state.reconcileRefs)

  useLayoutEffect(() => {
    if (!rows) return
    reconcileRefs(previous.current, rows)
    previous.current = rows
  }, [rows, reconcileRefs])
}
