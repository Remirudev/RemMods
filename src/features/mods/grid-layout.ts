import type { GridLayout } from './types'

const CARD_MIN_WIDTH = 210
// The lines under the picture. The picture takes the rest of the card: a square at the app's normal
// scale, a little less at larger ones, where these lines grow.
const CARD_TEXT_HEIGHT = 90

export const GRID_GAP = 12

// As many cards as fit, sharing the width; before the width is known it counts as one card.
export function gridLayout(width: number): GridLayout {
  const columns = Math.max(1, Math.floor((width + GRID_GAP) / (CARD_MIN_WIDTH + GRID_GAP)))
  const cardWidth = Math.max(CARD_MIN_WIDTH, Math.floor((width + GRID_GAP) / columns) - GRID_GAP)
  const cardHeight = cardWidth + CARD_TEXT_HEIGHT
  return { columns, cardHeight, lineHeight: cardHeight + GRID_GAP }
}
