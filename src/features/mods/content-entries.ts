import type { TFunction } from 'i18next'
import type { AssetEntry, AssetOrigin } from '../../lib/ipc/types'
import { characterLabel, slotLabel } from './content-hints'

export const ENTRY_RENDER_LIMIT = 300

export function sharedLabel(
  origin: AssetOrigin,
  count: number,
  t: TFunction<'mods'>,
): string | null {
  if (count === 0) return null
  if (origin === 'replaces') return t(($) => $.content.conflictReplaces, { count })
  if (origin === 'adds') return t(($) => $.content.conflictAdds, { count })
  return t(($) => $.content.sharedWith, { count })
}

export function originLabelKey(origin: AssetOrigin): AssetOrigin {
  switch (origin) {
    case 'adds':
      return 'adds'
    case 'replaces':
      return 'replaces'
    case 'unknown':
      return 'unknown'
  }
}

function labelsOf(entry: AssetEntry, t: TFunction<'mods'>): string[] {
  const labels = [entry.path]
  if (entry.character) labels.push(characterLabel(entry.character, t), entry.character.code)
  if (entry.slot) labels.push(slotLabel(entry.slot, t), entry.slot.code)
  return labels
}

export function filterEntries(
  entries: readonly AssetEntry[],
  needle: string,
  t: TFunction<'mods'>,
  locale: string,
): AssetEntry[] {
  const lowered = needle.trim().toLocaleLowerCase(locale)
  if (lowered === '') return [...entries]
  return entries.filter((entry) =>
    labelsOf(entry, t).some((label) => label.toLocaleLowerCase(locale).includes(lowered)),
  )
}
