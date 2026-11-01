import { describe, expect, it } from 'vitest'
import { makeGameStatus, TEST_INSTALL } from '../../test/fixtures'
import {
  checkCommand,
  checkGameStatus,
  checkMantineCss,
  checkModsListWithoutGame,
  checkModsTable,
  checkOnboarding,
} from './smoke-checks'

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

describe('checkGameStatus', () => {
  it('accepts a fixture location and an unset one', () => {
    const fixture = makeGameStatus({
      location: { kind: 'ready', install: { ...TEST_INSTALL, source: 'fixture' } },
    })
    expect(checkGameStatus({ ok: true, value: fixture }).ok).toBe(true)
    const notSet = makeGameStatus({ location: { kind: 'notSet' } })
    expect(checkGameStatus({ ok: true, value: notSet }).ok).toBe(true)
  })

  it('fails for a real or missing location and for a rejected command', () => {
    const steam = makeGameStatus()
    expect(checkGameStatus({ ok: true, value: steam }).ok).toBe(false)
    const missing = makeGameStatus({ location: { kind: 'missing', path: '/gone' } })
    expect(checkGameStatus({ ok: true, value: missing }).ok).toBe(false)
    expect(checkGameStatus({ ok: false, error: 'boom' })).toEqual({
      name: 'gameStatus',
      ok: false,
      detail: 'boom',
    })
  })
})

describe('DOM checks', () => {
  function documentWithCount(selectorCount: Record<string, number>): Document {
    return {
      querySelectorAll: (selector: string) => ({ length: selectorCount[selector] ?? 0 }),
      querySelector: (selector: string) => ((selectorCount[selector] ?? 0) > 0 ? {} : null),
    } as unknown as Document
  }

  it('checkModsTable needs at least one row', () => {
    expect(checkModsTable(documentWithCount({ '[data-testid="mods-row"]': 3 })).ok).toBe(true)
    expect(checkModsTable(documentWithCount({})).ok).toBe(false)
  })

  it('checkOnboarding needs the onboarding panel', () => {
    expect(checkOnboarding(documentWithCount({ '[data-testid="onboarding"]': 1 })).ok).toBe(true)
    expect(checkOnboarding(documentWithCount({})).ok).toBe(false)
  })
})

describe('checkModsListWithoutGame', () => {
  it('passes only for a rejection with the gameNotSet kind', () => {
    expect(checkModsListWithoutGame({ rejected: true, kind: 'gameNotSet' }).ok).toBe(true)
    expect(checkModsListWithoutGame({ rejected: true, kind: 'internal' }).ok).toBe(false)
    expect(checkModsListWithoutGame({ rejected: true, kind: null }).ok).toBe(false)
    expect(checkModsListWithoutGame({ rejected: false }).ok).toBe(false)
  })
})
