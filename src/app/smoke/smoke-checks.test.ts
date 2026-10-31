import { describe, expect, it } from 'vitest'
import { checkCommand, checkMantineCss } from './smoke-checks'

function documentWith(variables: Record<string, string>): Document {
  const style = { getPropertyValue: (name: string) => variables[name] ?? '' }
  const view = { getComputedStyle: () => style }
  return { defaultView: view, documentElement: {} } as unknown as Document
}

describe('checkMantineCss', () => {
  it('passes once the runtime theme variables apply', () => {
    const doc = documentWith({
      '--mantine-color-dark-7': ' #0B0B0F',
      '--qc-text-2': '#c9c9d2',
    })
    expect(checkMantineCss(doc).ok).toBe(true)
  })

  it('fails with only the static stylesheet, as when the CSP blocks runtime styles', () => {
    const doc = documentWith({
      '--mantine-color-body': '#242424',
      '--mantine-color-dark-7': '#242424',
    })
    expect(checkMantineCss(doc).ok).toBe(false)
  })

  it('fails when the resolver variables are missing', () => {
    const doc = documentWith({ '--mantine-color-dark-7': '#0b0b0f' })
    expect(checkMantineCss(doc).ok).toBe(false)
  })

  it('fails without a window to compute styles in', () => {
    const doc = { defaultView: null, documentElement: {} } as unknown as Document
    expect(checkMantineCss(doc).ok).toBe(false)
  })
})

describe('checkCommand', () => {
  it('passes a resolved command and keeps the reason of a rejected one', () => {
    expect(checkCommand('appReady', { ok: true, value: null }).ok).toBe(true)
    expect(checkCommand('appReady', { ok: false, error: 'not allowed' })).toEqual({
      name: 'appReady',
      ok: false,
      detail: 'not allowed',
    })
  })
})
