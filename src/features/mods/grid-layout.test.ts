import { describe, expect, it } from 'vitest'
import { gridLayout } from './grid-layout'

describe('gridLayout', () => {
  it('fits as many cards as the width allows', () => {
    expect(gridLayout(432).columns).toBe(2)
    expect(gridLayout(431).columns).toBe(1)
    expect(gridLayout(1834).columns).toBe(8)
  })

  it('gives the picture a square above the text lines', () => {
    expect(gridLayout(772)).toEqual({ columns: 3, cardHeight: 339, lineHeight: 351 })
  })

  it('keeps a whole card while the pane is narrower than one, or not measured yet', () => {
    expect(gridLayout(0)).toEqual({ columns: 1, cardHeight: 300, lineHeight: 312 })
    expect(gridLayout(100).cardHeight).toBe(300)
  })
})
