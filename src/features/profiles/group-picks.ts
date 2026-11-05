import type { ModRow, Profile, ProfileEntryDto } from '../../lib/ipc/types'
import { keyName } from './diff-groups'
import type { ModPick, PickCounts, PickItem, Picks } from './types'

// The profile code's "on" (core `wants_disable`): enabled, partly enabled, or a LogicMods folder
// disabled by its name only whose paks still load.
export function isRowOn(row: ModRow): boolean {
  return (
    row.state === 'enabled' ||
    row.state === 'partial' ||
    row.issues.some((issue) => issue.kind === 'logicDisabledButLoaded')
  )
}

// What "Start from current state" picks, like a saved state: every row that can be toggled, on
// or off as it is now. Ambiguous rows share a key with another row, so they are left alone.
export function picksFromRows(rows: readonly ModRow[]): Map<string, ModPick> {
  const picks = new Map<string, ModPick>()
  for (const row of rows) {
    if (row.ambiguous || row.toggleBlock !== null) continue
    if (isRowOn(row)) picks.set(row.key, 'on')
    else if (row.state === 'disabled') picks.set(row.key, 'off')
  }
  return picks
}

export function picksFromProfile(profile: Profile | null): Map<string, ModPick> {
  const picks = new Map<string, ModPick>()
  for (const entry of profile?.mods ?? []) {
    picks.set(entry.key, entry.state === 'enabled' ? 'on' : 'off')
  }
  return picks
}

// A copy with `keys` set to `pick`; "leave alone" removes them from the list.
export function withPicks(
  picks: Picks,
  keys: readonly string[],
  pick: ModPick,
): Map<string, ModPick> {
  const next = new Map(picks)
  for (const key of keys) {
    if (pick === 'skip') next.delete(key)
    else next.set(key, pick)
  }
  return next
}

export function countPicks(picks: Picks): PickCounts {
  let on = 0
  let off = 0
  for (const pick of picks.values()) {
    if (pick === 'on') on += 1
    else if (pick === 'off') off += 1
  }
  return { on, off }
}

// The list the profile stores, in key order; mods left alone are not listed.
export function pickEntries(picks: Picks): ProfileEntryDto[] {
  const entries: ProfileEntryDto[] = []
  for (const [key, pick] of picks) {
    if (pick === 'skip') continue
    entries.push({ key, state: pick === 'on' ? 'enabled' : 'disabled' })
  }
  return entries.sort((a, b) => (a.key < b.key ? -1 : a.key > b.key ? 1 : 0))
}

// Installed mods by name, then the listed mods that are not installed (kept, so editing a group
// never drops them silently). A key shared by several rows shows once.
export function pickItems(
  rows: readonly ModRow[],
  listed: readonly string[],
  locale: string,
): PickItem[] {
  const collator = new Intl.Collator(locale, { sensitivity: 'base', numeric: true })
  const seen = new Set<string>()
  const installed: PickItem[] = []
  for (const row of rows) {
    if (seen.has(row.key)) continue
    seen.add(row.key)
    installed.push({ key: row.key, name: row.name, on: isRowOn(row) })
  }
  installed.sort((a, b) => collator.compare(a.name, b.name))
  const missing = listed
    .filter((key) => !seen.has(key))
    .map((key): PickItem => ({ key, name: keyName(key), on: null }))
    .sort((a, b) => collator.compare(a.name, b.name))
  return [...installed, ...missing]
}

export function matchesPickSearch(item: PickItem, needle: string, locale: string): boolean {
  if (needle === '') return true
  return [item.name, item.key].some((value) => value.toLocaleLowerCase(locale).includes(needle))
}
