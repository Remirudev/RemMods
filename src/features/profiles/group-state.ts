import type { ModRow, Profile, ProfileSummary, ProfilesView } from '../../lib/ipc/types'
import { outfitConflicts } from '../../shared/outfit-conflicts'
import { applyClick } from '../../shared/selection'
import type { ClickModifiers, SelectionState } from '../../shared/types'
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

// A click on a group's name: Ctrl adds or removes it and Shift takes everything from the last click
// on, like a file list. A plain click on the only picked group shows all mods again.
export function picksAfterClick(
  picked: readonly string[],
  anchor: string | null,
  ids: readonly string[],
  id: string,
  modifiers: ClickModifiers,
): SelectionState {
  const plain = !modifiers.shift && !modifiers.ctrl
  if (plain && picked.length === 1 && picked[0] === id) return { selection: [], anchor: null }
  return applyClick(picked, anchor, ids, id, modifiers)
}

function pickedItems(items: readonly GroupItem[], picked: readonly string[]): GroupItem[] {
  return items.filter((item) => picked.includes(item.summary.profile.id))
}

// A switch acts on a selection the way a file list does: on all picked groups when its group is one
// of several, else on its group alone.
export function switchTargets(
  items: readonly GroupItem[],
  picked: readonly string[],
  item: GroupItem,
): GroupItem[] {
  const among = picked.length > 1 && picked.includes(item.summary.profile.id)
  return among ? pickedItems(items, picked) : [item]
}

// All on, All off: the picked groups when there are several, else every group. One picked group is
// the list's filter, not a selection.
export function bulkTargets(items: readonly GroupItem[], picked: readonly string[]): GroupItem[] {
  return picked.length > 1 ? pickedItems(items, picked) : [...items]
}

// Up to two letters for the collapsed sidebar: the first letters of the first two words.
export function groupInitials(title: string): string {
  const words = title.trim().split(/\s+/).filter(Boolean)
  const first = words[0] ?? ''
  const letters =
    words.length > 1 ? [first, words[1] ?? ''].map((word) => [...word][0]) : [...first].slice(0, 2)
  return letters.join('').toLocaleUpperCase()
}
