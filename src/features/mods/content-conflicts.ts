import type { ContentConflictLink, ModRow } from '../../lib/ipc/types'

// The links the "replaces same files" badge, its column and its filter count: only shared
// upper-body or full-body clothing. The details drawer lists every link.
export function outfitConflicts(row: ModRow): ContentConflictLink[] {
  return row.contentConflicts.filter((link) => link.outfit > 0)
}
