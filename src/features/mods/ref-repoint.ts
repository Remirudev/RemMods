import type { ModRow } from '../../lib/ipc/types'

// A LogicMods toggle renames the folder, and the ref is built from the folder name, so the same
// mod comes back with a new ref. The key stays; rows whose key is not unique are never matched.
export function movedRefs(
  previous: readonly ModRow[],
  next: readonly ModRow[],
): ReadonlyMap<string, string> {
  const present = new Set(next.map((row) => row.modRef))
  const byKey = new Map<string, string | null>()
  for (const row of next) {
    byKey.set(row.key, byKey.has(row.key) ? null : row.modRef)
  }
  const moved = new Map<string, string>()
  for (const row of previous) {
    if (present.has(row.modRef)) continue
    const target = byKey.get(row.key)
    if (target) moved.set(row.modRef, target)
  }
  return moved
}

export function repointRef(
  ref: string,
  moved: ReadonlyMap<string, string>,
  next: readonly ModRow[],
): string | null {
  if (next.some((row) => row.modRef === ref)) return ref
  return moved.get(ref) ?? null
}
