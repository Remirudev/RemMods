import { describe, expect, it } from 'vitest'
import { previewBox } from './preview-box'

const VIEWPORT = { width: 2000, height: 1000 }
const ICON = { width: 192, height: 192 }

function anchorAt(left: number, top = 400, size = 200) {
  return { left, right: left + size, top, bottom: top + size }
}

describe('previewBox', () => {
  it('opens on the side with more room, beside the picture', () => {
    expect(previewBox(anchorAt(100), ICON, VIEWPORT).left).toBe(324)
    expect(previewBox(anchorAt(1700), ICON, VIEWPORT).left).toBe(1700 - 24 - 576)
  })

  it('stops growing a small picture at three times its size', () => {
    const box = previewBox(anchorAt(100), ICON, VIEWPORT)
    expect([box.width, box.height]).toEqual([576, 576])
  })

  it('keeps a large picture inside the window and its shape', () => {
    const box = previewBox(anchorAt(100), { width: 4000, height: 2000 }, VIEWPORT)
    expect([box.width, box.height]).toEqual([1660, 830])
  })

  it('fits a tall picture to the window height', () => {
    const box = previewBox(anchorAt(100), { width: 1000, height: 4000 }, VIEWPORT)
    expect(box.height).toBe(968)
    expect(box.top).toBe(16)
  })

  it('stays beside the picture vertically but inside the window', () => {
    expect(previewBox(anchorAt(100, 400), ICON, VIEWPORT).top).toBe(212)
    expect(previewBox(anchorAt(100, 0), ICON, VIEWPORT).top).toBe(16)
    expect(previewBox(anchorAt(100, 800), ICON, VIEWPORT).top).toBe(1000 - 16 - 576)
  })
})
