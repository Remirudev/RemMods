import { describe, expect, it } from 'vitest'
import { makeModRow } from '../../test/fixtures'
import { movedRefs, repointRef } from './ref-repoint'

const before = [makeModRow('Tool', { kind: 'logic' }), makeModRow('Alpha')]
const renamed = makeModRow('Tool', { kind: 'logic', modRef: 'm:.disabled_tool' })

describe('movedRefs', () => {
  it('maps a vanished ref to the row with the same key', () => {
    const moved = movedRefs(before, [renamed, before[1]!])
    expect(moved.get('m:tool')).toBe('m:.disabled_tool')
    expect(moved.has('m:alpha')).toBe(false)
  })

  it('does not match rows whose key is shared', () => {
    const twin = makeModRow('Tool', { kind: 'logic', modRef: 'm:other' })
    expect(movedRefs(before, [renamed, twin]).size).toBe(0)
  })

  it('does not map a row that is simply gone', () => {
    expect(movedRefs(before, [before[1]!]).size).toBe(0)
  })
})

describe('repointRef', () => {
  it('keeps a present ref, follows a moved one and drops a gone one', () => {
    const next = [renamed, before[1]!]
    const moved = movedRefs(before, next)
    expect(repointRef('m:alpha', moved, next)).toBe('m:alpha')
    expect(repointRef('m:tool', moved, next)).toBe('m:.disabled_tool')
    expect(repointRef('m:nope', moved, next)).toBeNull()
  })
})
