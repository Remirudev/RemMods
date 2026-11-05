import type { CharacterHint, ModItem, Slot } from '../../lib/ipc/types'
import type { ItemGroups } from './types'

const SLOT_ORDER: readonly Slot[] = [
  'fullBody',
  'upper',
  'lower',
  'bottom',
  'hair',
  'fullHead',
  'head',
  'hat',
  'face',
  'facePaint',
  'faceHair',
  'eyeMakeup',
  'eyes',
  'cheek',
  'lips',
  'glasses',
  'accessory',
  'shoes',
  'extra',
]

// Outfits lead, as in the game's own menu; codes no slot is known for come last.
function slotRank(item: ModItem): number {
  const rank = item.slot.slot === null ? -1 : SLOT_ORDER.indexOf(item.slot.slot)
  return rank === -1 ? SLOT_ORDER.length : rank
}

function compareItems(a: ModItem, b: ModItem): number {
  return slotRank(a) - slotRank(b) || a.key.localeCompare(b.key)
}

// Each character once, by name; the shared sets (no name) after the named ones.
export function charactersOf(items: readonly ModItem[], locale: string): CharacterHint[] {
  const byCode = new Map(items.map((item) => [item.character.code, item.character]))
  return [...byCode.values()].sort(
    (a, b) =>
      Number(a.name === null) - Number(b.name === null) ||
      (a.name ?? a.code).localeCompare(b.name ?? b.code, locale),
  )
}

export function chosenCharacter(
  characters: readonly CharacterHint[],
  wanted: string | null,
): string | null {
  const known = characters.some((character) => character.code === wanted)
  return known ? wanted : (characters[0]?.code ?? null)
}

// What a mod changes the look of ships a new picture. The rest supports it, like hidden shoes.
// A mod without any picture has no such split: everything counts.
export function itemGroups(items: readonly ModItem[], character: string | null): ItemGroups {
  const own = items.filter((item) => item.character.code === character).sort(compareItems)
  const pictured = own.filter((item) => item.modPicture !== null)
  if (pictured.length === 0) return { main: own, other: [] }
  return { main: pictured, other: own.filter((item) => item.modPicture === null) }
}

// The name inside the key, for items the game has no text for.
export function internalName(item: ModItem): string {
  return item.key.slice(item.character.code.length + 1)
}

// A mod's look at a glance: its first picture in outfit order, for this character if it has one.
export function coverPicture(items: readonly ModItem[], character: string | null): string | null {
  const pictured = items.filter((item) => item.modPicture !== null).sort(compareItems)
  const own = pictured.find((item) => item.character.code === character)
  return (own ?? pictured[0])?.modPicture ?? null
}
