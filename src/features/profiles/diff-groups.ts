import type { ProfileDiff } from '../../lib/ipc/types'
import type { DiffGroups } from './types'

export function groupDiff(diff: ProfileDiff): DiffGroups {
  return {
    enable: diff.changes.filter((change) => change.enable),
    disable: diff.changes.filter((change) => !change.enable),
    skipped: diff.skipped,
    missing: diff.missing,
    hasChanges: diff.changes.length > 0,
  }
}

// Keys read "Standard:Name": the part after the first colon is what the user knows.
export function keyName(key: string): string {
  const colon = key.indexOf(':')
  return colon < 0 ? key : key.slice(colon + 1)
}
