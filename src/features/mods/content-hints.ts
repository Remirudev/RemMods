import type { TFunction } from 'i18next'
import type { CharacterHint, SlotHint } from '../../lib/ipc/types'

const WOMEN_CODE = 'cf0'
const MEN_CODE = 'cm0'

// Only confirmed names arrive from the backend; every other code is shown raw.
export function characterLabel(hint: CharacterHint, t: TFunction<'mods'>): string {
  if (hint.name !== null) return hint.name
  if (hint.common) return t(($) => $.content.allCharacters)
  if (hint.code === WOMEN_CODE) return t(($) => $.content.allWomen)
  if (hint.code === MEN_CODE) return t(($) => $.content.allMen)
  return hint.code
}

const CARD_CHARACTERS = 3

// A card's one line: the first few names, then how many more.
export function characterSummary(
  hints: readonly CharacterHint[],
  t: TFunction<'mods'>,
  locale: string,
): string {
  const labels = hints
    .map((hint) => characterLabel(hint, t))
    .sort((a, b) => a.localeCompare(b, locale))
  const names = labels.slice(0, CARD_CHARACTERS).join(', ')
  const count = labels.length - CARD_CHARACTERS
  return count > 0 ? t(($) => $.grid.moreCharacters, { names, count }) : names
}

export function slotLabel(hint: SlotHint, t: TFunction<'mods'>): string {
  const slot = hint.slot
  if (slot === null) return hint.code
  return t(($) => $.slots[slot])
}

export function isRawCharacter(hint: CharacterHint): boolean {
  return hint.name === null && !hint.common && hint.code !== WOMEN_CODE && hint.code !== MEN_CODE
}
