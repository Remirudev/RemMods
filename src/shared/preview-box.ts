import type { PictureSize, PreviewAnchor, PreviewBox } from './types'

const GAP = 24
const MARGIN = 16
// Stored game pictures are small; further up they only get blurrier.
const MAX_UPSCALE = 3

// Beside the picture, on the side with more room, and as near to its height as the window allows.
export function previewBox(
  anchor: PreviewAnchor,
  natural: PictureSize,
  viewport: PictureSize,
): PreviewBox {
  const roomLeft = anchor.left - GAP - MARGIN
  const roomRight = viewport.width - anchor.right - GAP - MARGIN
  const leftward = roomLeft > roomRight
  const scale = Math.min(
    MAX_UPSCALE,
    Math.max(roomLeft, roomRight) / natural.width,
    (viewport.height - 2 * MARGIN) / natural.height,
  )
  const width = Math.round(natural.width * scale)
  const height = Math.round(natural.height * scale)
  const centered = (anchor.top + anchor.bottom - height) / 2
  return {
    left: leftward ? anchor.left - GAP - width : anchor.right + GAP,
    top: Math.max(MARGIN, Math.min(centered, viewport.height - MARGIN - height)),
    width,
    height,
  }
}
