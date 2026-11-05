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

export function slotLabel(hint: SlotHint, t: TFunction<'mods'>): string {
  const slot = hint.slot
  if (slot === null) return hint.code
  return t(($) => $.slots[slot])
}

export function isRawCharacter(hint: CharacterHint): boolean {
  return hint.name === null && !hint.common && hint.code !== WOMEN_CODE && hint.code !== MEN_CODE
}
