import type { ModRow, Profile, ProfileSummary, ProfilesView } from '../../lib/ipc/types'
import { outfitConflicts } from '../../shared/outfit-conflicts'
import { profileTitle, splitProfiles } from './profile-label'
import type { Clash, GroupItem } from './types'

// The keys a group switches on.
export function onKeysOf(profile: Profile): string[] {
  return profile.mods.filter((entry) => entry.state === 'enabled').map((entry) => entry.key)
}

// A group's switch is on while the mods match it. An empty additive group matches anything, so
// it stays off; an empty exclusive group ("everything off") is a real state and can be on.
export function isLit(summary: ProfileSummary): boolean {
  const { profile } = summary
  return summary.matchesCurrent && (profile.mods.length > 0 || profile.mode === 'exclusive')
}

// Pairs among `keys` that replace the same files (pak names or outfit items), each pair once.
export function groupClashes(rows: readonly ModRow[], keys: readonly string[]): Clash[] {
  const wanted = new Set(keys)
  const pairs = new Map<string, Clash>()
  for (const row of rows) {
    if (!wanted.has(row.key)) continue
    for (const link of [...row.conflictsWith, ...outfitConflicts(row)]) {
      if (!wanted.has(link.key) || link.key === row.key) continue
      const [first, second] = row.key < link.key ? [row, link] : [link, row]
      const id = `${first.key}\n${second.key}`
      if (!pairs.has(id)) pairs.set(id, { first: first.name, second: second.name })
    }
  }
  return [...pairs.values()].sort(
    (a, b) => a.first.localeCompare(b.first) || a.second.localeCompare(b.second),
  )
}

export function groupItems(
  view: ProfilesView | null | undefined,
  rows: readonly ModRow[],
  locale: string,
  unnamed: string,
): GroupItem[] {
  return splitProfiles(view?.profiles ?? [], locale).user.map((summary) => {
    const onKeys = onKeysOf(summary.profile)
    return {
      summary,
      title: profileTitle(summary.profile, unnamed),
      lit: isLit(summary),
      onKeys,
      clashes: groupClashes(rows, onKeys),
    }
  })
}

// Up to two letters for the collapsed sidebar: the first letters of the first two words.
export function groupInitials(title: string): string {
  const words = title.trim().split(/\s+/).filter(Boolean)
  const first = words[0] ?? ''
  const letters =
    words.length > 1 ? [first, words[1] ?? ''].map((word) => [...word][0]) : [...first].slice(0, 2)
  return letters.join('').toLocaleUpperCase()
}
