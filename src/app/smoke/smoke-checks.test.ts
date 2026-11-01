import { describe, expect, it } from 'vitest'
import { makeGameStatus, makeModRow, makeSnapshot, TEST_INSTALL } from '../../test/fixtures'
import {
  checkCommand,
  checkEmptyProfiles,
  checkGameStatus,
  checkMantineCss,
  checkModsListWithoutGame,
  checkModsTable,
  checkOnboarding,
  checkToggleRoundTrip,
  expectRowState,
  hasToggleFixture,
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

describe('checkEmptyProfiles', () => {
  const view = { profiles: [], lastApplied: null, readOnly: false, problem: null }

  it('passes an empty, writable view', () => {
    expect(checkEmptyProfiles({ ok: true, value: view }).ok).toBe(true)
  })

  it('fails a read-only view and a rejected command', () => {
    expect(checkEmptyProfiles({ ok: true, value: { ...view, readOnly: true } }).ok).toBe(false)
    expect(checkEmptyProfiles({ ok: false, error: 'denied' }).detail).toBe('denied')
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

describe('toggle round trip', () => {
  const alpha = makeModRow('Alpha', { key: 'Standard:Alpha' })
  const tool = makeModRow('Tool', { key: 'Logic:Tool', kind: 'logic' })

  it('needs both fixture rows', () => {
    expect(hasToggleFixture(makeSnapshot([alpha, tool]))).toBe(true)
    expect(hasToggleFixture(makeSnapshot([alpha]))).toBe(false)
    expect(hasToggleFixture(makeSnapshot([]))).toBe(false)
  })

  it('compares the state of a row by key', () => {
    const snapshot = makeSnapshot([{ ...alpha, state: 'disabled' }])
    expect(expectRowState(snapshot, 'Standard:Alpha', 'disabled')).toBeNull()
    expect(expectRowState(snapshot, 'Standard:Alpha', 'enabled')).toContain('expected enabled')
    expect(expectRowState(snapshot, 'Logic:Tool', 'enabled')).toContain('missing')
  })

  it('passes with no failures, passes as skipped without a fixture, and fails otherwise', () => {
    expect(checkToggleRoundTrip([]).ok).toBe(true)
    expect(checkToggleRoundTrip(null)).toEqual({
      name: 'toggleRoundTrip',
      ok: true,
      detail: 'no fixture',
    })
    const failed = checkToggleRoundTrip(['a', 'b'])
    expect(failed.ok).toBe(false)
    expect(failed.detail).toBe('a\nb')
  })
})
