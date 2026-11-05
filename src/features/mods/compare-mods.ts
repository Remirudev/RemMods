import type { ModKind, ModRow } from '../../lib/ipc/types'
import { outfitConflicts } from '../../shared/outfit-conflicts'
import { displayState } from './display-state'
import type { ModColumnId, ModDisplayState } from './types'

const STATE_ORDER: Record<ModDisplayState, number> = {
  enabled: 0,
  partial: 1,
  disabledLoaded: 2,
  disabled: 3,
  noPakFiles: 4,
  unknown: 5,
}

const KIND_ORDER: Record<ModKind, number> = {
  standard: 0,
  legacy: 1,
  logic: 2,
  loose: 3,
}

const collators = new Map<string, Intl.Collator>()

// Numeric so "Mod 2" sorts before "Mod 10"; base sensitivity so case and accents do not split
// otherwise equal names.
export function modCollator(locale: string): Intl.Collator {
  let collator = collators.get(locale)
  if (!collator) {
    collator = new Intl.Collator(locale, { numeric: true, sensitivity: 'base' })
    collators.set(locale, collator)
  }
  return collator
}

function compareColumn(a: ModRow, b: ModRow, column: ModColumnId, collator: Intl.Collator): number {
  switch (column) {
    case 'name':
      return collator.compare(a.name, b.name)
    case 'state':
      return STATE_ORDER[displayState(a)] - STATE_ORDER[displayState(b)]
    case 'kind':
      return KIND_ORDER[a.kind] - KIND_ORDER[b.kind]
    case 'version':
      return collator.compare(a.info.version, b.info.version)
    case 'category':
      return collator.compare(a.info.category, b.info.category)
    case 'size':
      return (a.sizeBytes ?? -1) - (b.sizeBytes ?? -1)
    case 'conflicts':
      return a.conflictsWith.length - b.conflictsWith.length
    case 'contentConflicts':
      return outfitConflicts(a).length - outfitConflicts(b).length
    case 'issues':
      return a.issues.length - b.issues.length
  }
}

function compareCodeUnits(a: string, b: string): number {
  if (a === b) return 0
  return a < b ? -1 : 1
}

// Ascending; ties fall back to the name and then the ModRef, so the order never depends on how
// the rows arrived.
export function compareMods(
  a: ModRow,
  b: ModRow,
  column: ModColumnId,
  collator: Intl.Collator,
): number {
  return (
    compareColumn(a, b, column, collator) ||
    collator.compare(a.name, b.name) ||
    compareCodeUnits(a.modRef, b.modRef)
  )
}
