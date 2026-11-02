import { describe, expect, it } from 'vitest'
import { makeCandidate, makeInspection, makeIssue } from '../../test/install-fixtures'
import {
  buildStartRequest,
  carryChoices,
  initialChoices,
  keepClashPath,
  pickVariant,
  setAutoFix,
  setReplace,
  startBlockers,
  toggleCandidate,
  withName,
} from './install-model'

const EXISTING = {
  modRef: 'm:cool',
  key: 'Standard:Cool',
  rel: 'Mods/Cool',
  relation: 'sameName' as const,
  ambiguous: false,
}

const VARIANTS = makeInspection(
  [
    makeCandidate(0, 'Cool A', { group: 0, selected: true }),
    makeCandidate(1, 'Cool B', { group: 0, selected: false }),
    makeCandidate(2, 'Extra', { selected: true }),
  ],
  { groups: [{ id: 0, candidates: [0, 1] }] },
)

describe('initialChoices', () => {
  it('follows the backend: first variant and every separate mod, nothing confirmed', () => {
    expect(initialChoices(VARIANTS)).toEqual({
      selected: [0, 2],
      replace: [],
      clashKeeps: {},
      autoFix: true,
    })
  })
})

describe('choosing', () => {
  it('toggles a separate mod and forgets its replace switch', () => {
    const start = setReplace(initialChoices(VARIANTS), 2, true)
    const off = toggleCandidate(start, 2)
    expect(off.selected).toEqual([0])
    expect(off.replace).toEqual([])
    expect(toggleCandidate(off, 2).selected).toEqual([0, 2])
  })

  it('keeps exactly one variant of a group', () => {
    const picked = pickVariant(VARIANTS, initialChoices(VARIANTS), 1)
    expect(picked.selected).toEqual([2, 1])
    expect(pickVariant(VARIANTS, picked, 1).selected).toEqual([2, 1])
  })

  it('records a clash choice per group', () => {
    const choices = keepClashPath(keepClashPath(initialChoices(VARIANTS), 0, 1), 3, 0)
    expect(choices.clashKeeps).toEqual({ 0: 1, 3: 0 })
  })
})

describe('startBlockers', () => {
  const single = makeInspection([makeCandidate(0, 'Cool')])
  const choices = initialChoices(single)

  it('is empty for a plain install', () => {
    expect(startBlockers(single, choices)).toEqual([])
  })

  it('blocks on a blocked inspection', () => {
    const blocked = makeInspection([makeCandidate(0, 'Cool')], {
      blocked: true,
      issues: [makeIssue('noPayload', 'block')],
    })
    expect(startBlockers(blocked, initialChoices(blocked))).toEqual(['blocked'])
  })

  it('blocks without a selection', () => {
    expect(startBlockers(single, toggleCandidate(choices, 0))).toEqual(['noSelection'])
  })

  it('wants the replace switch for an existing mod and refuses an ambiguous one', () => {
    const replacing = makeInspection([makeCandidate(0, 'Cool', { existing: EXISTING })])
    const start = initialChoices(replacing)
    expect(startBlockers(replacing, start)).toEqual(['replaceNotConfirmed'])
    expect(startBlockers(replacing, setReplace(start, 0, true))).toEqual([])

    const ambiguous = makeInspection([
      makeCandidate(0, 'Cool', { existing: { ...EXISTING, ambiguous: true } }),
    ])
    expect(startBlockers(ambiguous, setReplace(initialChoices(ambiguous), 0, true))).toEqual([
      'ambiguous',
    ])
  })

  it('wants every case clash resolved', () => {
    const clashing = makeInspection([makeCandidate(0, 'Cool')], {
      clashes: [{ id: 0, paths: ['A.pak', 'a.pak'] }],
      issues: [makeIssue('caseClash', 'warn')],
    })
    const start = initialChoices(clashing)
    expect(startBlockers(clashing, start)).toEqual(['unresolvedClash'])
    expect(startBlockers(clashing, keepClashPath(start, 0, 0))).toEqual([])
  })

  it('wants automatic renaming for invalid names', () => {
    const invalid = makeInspection([makeCandidate(0, 'Co:ol', { fixedName: 'Co_ol' })], {
      issues: [makeIssue('invalidName', 'warn')],
    })
    const start = initialChoices(invalid)
    expect(startBlockers(invalid, start)).toEqual([])
    expect(startBlockers(invalid, setAutoFix(start, false))).toEqual(['invalidNames'])
  })

  it('refuses when there is not enough space', () => {
    const full = makeInspection([makeCandidate(0, 'Cool')], {
      issues: [makeIssue('noSpace', 'warn')],
    })
    expect(startBlockers(full, initialChoices(full))).toEqual(['noSpace'])
  })
})

describe('buildStartRequest', () => {
  it('sends picks with replace flags, clash picks and the auto-fix choice', () => {
    const inspection = makeInspection(
      [makeCandidate(0, 'Cool', { existing: EXISTING }), makeCandidate(1, 'Other')],
      { clashes: [{ id: 4, paths: ['A.pak', 'a.pak'] }] },
    )
    const choices = keepClashPath(setReplace(initialChoices(inspection), 0, true), 4, 1)
    expect(buildStartRequest(inspection, setAutoFix(choices, false))).toEqual({
      inspection: inspection.id,
      picks: [
        { id: 0, replace: true },
        { id: 1, replace: false },
      ],
      clashes: [{ id: 4, keep: 1 }],
      autoFix: false,
      applyWhileRunning: false,
    })
  })

  it('never marks a mod without an installed match as a replace', () => {
    const inspection = makeInspection([makeCandidate(0, 'Cool')])
    const choices = setReplace(initialChoices(inspection), 0, true)
    expect(buildStartRequest(inspection, choices).picks).toEqual([{ id: 0, replace: false }])
  })

  it('carries applyWhileRunning for the retry', () => {
    const inspection = makeInspection([makeCandidate(0, 'Cool')])
    expect(buildStartRequest(inspection, initialChoices(inspection), true).applyWhileRunning).toBe(
      true,
    )
  })
})

describe('carryChoices and withName', () => {
  it('keeps decisions that still apply after a name edit', () => {
    const before = makeInspection([makeCandidate(0, 'Cool', { existing: EXISTING })], {
      clashes: [{ id: 0, paths: ['A', 'a'] }],
    })
    const decided = keepClashPath(setReplace(initialChoices(before), 0, true), 0, 1)
    const renamed = makeInspection([makeCandidate(0, 'Fresh')], {
      clashes: [{ id: 0, paths: ['A', 'a'] }],
    })
    expect(carryChoices(decided, renamed)).toEqual({
      selected: [0],
      replace: [],
      clashKeeps: { 0: 1 },
      autoFix: true,
    })
  })

  it('adds, replaces and clears name overrides', () => {
    const base = { encoding: 'auto' as const, root: null, names: [] }
    const one = withName(base, 2, 'New')
    expect(one.names).toEqual([{ id: 2, name: 'New' }])
    expect(withName(one, 2, 'Newer').names).toEqual([{ id: 2, name: 'Newer' }])
    expect(withName(one, 2, '').names).toEqual([])
  })
})
