import { describe, expect, it } from 'vitest'
import { buildTheme } from './build-theme'

describe('buildTheme', () => {
  it('uses the Quiet Canvas violet as the primary colour', () => {
    const theme = buildTheme()
    expect(theme.primaryColor).toBe('violet')
    expect(theme.primaryShade).toEqual({ light: 6, dark: 5 })
    expect(theme.colors?.violet?.[5]).toBe('#7c5cff')
  })

  it('replaces the dark scale with the near-black canvas at shade 7', () => {
    const theme = buildTheme()
    expect(theme.colors?.dark?.[7]).toBe('#0b0b0f')
    expect(theme.colors?.dark?.[0]).toBe('#ededf0')
  })

  it('uses the 6/8/12/16/20 radius scale and a 14px xs font size', () => {
    const theme = buildTheme()
    expect(theme.radius).toEqual({ xs: '6px', sm: '8px', md: '12px', lg: '16px', xl: '20px' })
    expect(theme.defaultRadius).toBe('md')
    expect(theme.fontSizes?.xs).toBe('14px')
  })

  it('never truncates badge labels', () => {
    const theme = buildTheme()
    expect(theme.components?.Badge?.styles).toEqual({ label: { overflow: 'visible' } })
  })

  it('applies the radius and density parameters', () => {
    const compact = buildTheme({ radius: 'sm', density: 'compact' })
    expect(compact.defaultRadius).toBe('sm')
    expect(compact.spacing?.md).toBe('12px')
    expect(buildTheme().spacing?.md).toBe('16px')
  })

  it('is pure: the same input gives an equal theme', () => {
    expect(buildTheme({ density: 'compact' })).toEqual(buildTheme({ density: 'compact' }))
  })
})
