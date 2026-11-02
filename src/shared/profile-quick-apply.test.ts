import { describe, expect, it, vi } from 'vitest'
import { makeProfile, makeSummary, makeView } from '../test/profile-fixtures'
import {
  openProfileApply,
  quickApplyChoices,
  quickApplyState,
  registerProfileApplyOpener,
} from './profile-quick-apply'

const VIEW = makeView(
  [
    makeSummary(makeProfile('p10', { name: 'Set 10', mode: 'exclusive' })),
    makeSummary(makeProfile('snap', { name: '', source: { kind: 'snapshot', of: 'p2' } })),
    makeSummary(makeProfile('p2', { name: 'Set 2', mods: [{ key: 'alpha', state: 'enabled' }] })),
    makeSummary(makeProfile('pa', { name: 'alpha set' })),
  ],
  { lastApplied: { profileId: 'p2', at: '2026-11-01T11:00:00.000Z' } },
)

describe('quick-apply choices', () => {
  it('leaves out snapshots and sorts by name, numbers by value', () => {
    expect(quickApplyChoices(VIEW, 'en').map((choice) => choice.id)).toEqual(['pa', 'p2', 'p10'])
    expect(quickApplyChoices(VIEW, 'en')[1]).toEqual({
      id: 'p2',
      name: 'Set 2',
      mode: 'additive',
      count: 1,
    })
  })

  it('knows the last applied profile and the read-only state', () => {
    expect(quickApplyState(VIEW, 'en').lastAppliedId).toBe('p2')
    expect(quickApplyState(VIEW, 'en').readOnly).toBe(false)
    expect(quickApplyState(makeView([], { readOnly: true }), 'en').readOnly).toBe(true)
    expect(quickApplyState(undefined, 'en')).toEqual({
      choices: [],
      lastAppliedId: null,
      readOnly: false,
    })
  })
})

describe('the profile apply opener', () => {
  it('opens nothing without a registered opener', () => {
    expect(openProfileApply('p2')).toBe(false)
  })

  it('calls the registered opener until it unregisters', () => {
    const open = vi.fn()
    const unregister = registerProfileApplyOpener(open)
    expect(openProfileApply('p2')).toBe(true)
    expect(open).toHaveBeenCalledWith('p2')
    unregister()
    expect(openProfileApply('p2')).toBe(false)
  })

  it('an old unregister leaves a newer opener in place', () => {
    const first = registerProfileApplyOpener(vi.fn())
    const second = vi.fn()
    const unregisterSecond = registerProfileApplyOpener(second)
    first()
    expect(openProfileApply('p10')).toBe(true)
    expect(second).toHaveBeenCalledWith('p10')
    unregisterSecond()
  })
})
