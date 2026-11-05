import type { Profile, ProfileSummary } from '../../lib/ipc/types'
import type { SplitProfiles } from './types'

// Imported profiles can have no name; the UI shows a translated word for them.
export function profileTitle(profile: Profile, unnamed: string): string {
  return profile.name.trim() === '' ? unnamed : profile.name
}

// The profile an automatic snapshot was taken for; null when that profile was deleted since.
export function snapshotTarget(snapshot: Profile, profiles: readonly Profile[]): Profile | null {
  if (snapshot.source.kind !== 'snapshot') return null
  const of = snapshot.source.of
  return profiles.find((profile) => profile.id === of) ?? null
}

export function splitProfiles(summaries: readonly ProfileSummary[], locale: string): SplitProfiles {
  const collator = new Intl.Collator(locale, { sensitivity: 'base', numeric: true })
  const user = summaries
    .filter((summary) => summary.profile.source.kind !== 'snapshot')
    .sort((a, b) => collator.compare(a.profile.name, b.profile.name))
  const snapshots = summaries
    .filter((summary) => summary.profile.source.kind === 'snapshot')
    .sort((a, b) => b.profile.created.localeCompare(a.profile.created))
  return { user, snapshots }
}

const UNITS: readonly [Intl.RelativeTimeFormatUnit, number][] = [
  ['day', 86_400_000],
  ['hour', 3_600_000],
  ['minute', 60_000],
]

export function relativeTime(iso: string, locale: string, now: number): string {
  const then = Date.parse(iso)
  if (Number.isNaN(then)) return iso
  const format = new Intl.RelativeTimeFormat(locale, { numeric: 'auto' })
  const delta = then - now
  for (const [unit, size] of UNITS) {
    if (Math.abs(delta) >= size) return format.format(Math.round(delta / size), unit)
  }
  return format.format(0, 'minute')
}
