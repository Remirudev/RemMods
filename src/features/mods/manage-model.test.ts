import { describe, expect, it } from 'vitest'
import { makeModRow } from '../../test/fixtures'
import { archiveSubject, canManage, leftoverSubject, manageBlock, rowSubject } from './manage-model'
import type { ManagedKind } from './types'

const ACTIONS = ['rename', 'trash', 'delete', 'metadata'] as const

describe('manage matrix (SPEC-P3 §5.1)', () => {
  const allowed: Record<ManagedKind, readonly (typeof ACTIONS)[number][]> = {
    folder: ['rename', 'trash', 'delete', 'metadata'],
    linked: [],
    loose: ['trash'],
    archiveInRoot: ['trash'],
    archiveInPaks: [],
    leftover: ['trash', 'delete'],
    unmanaged: [],
  }

  it.each(Object.entries(allowed))('allows exactly the listed actions for %s', (kind, actions) => {
    for (const action of ACTIONS) {
      expect(canManage({ kind: kind as ManagedKind }, action), `${kind} ${action}`).toBe(
        actions.includes(action),
      )
    }
  })

  it('names the reason of each refusal', () => {
    expect(manageBlock('loose', 'rename')).toBe('loose')
    expect(manageBlock('linked', 'trash')).toBe('linked')
    expect(manageBlock('archiveInPaks', 'trash')).toBe('outsideRoots')
    expect(manageBlock('archiveInRoot', 'delete')).toBe('archive')
    expect(manageBlock('leftover', 'rename')).toBe('leftover')
    expect(manageBlock('unmanaged', 'delete')).toBe('unmanaged')
  })
})

describe('subjects', () => {
  it('classifies rows', () => {
    expect(rowSubject(makeModRow('A')).kind).toBe('folder')
    expect(rowSubject(makeModRow('A', { kind: 'loose' })).kind).toBe('loose')
    expect(rowSubject(makeModRow('A', { toggleBlock: 'linked' })).kind).toBe('linked')
    expect(rowSubject(makeModRow('A', { issues: [{ kind: 'linkedMod' }] })).kind).toBe('linked')
    expect(rowSubject(makeModRow('A', { toggleBlock: 'ambiguous', ambiguous: true })).kind).toBe(
      'folder',
    )
  })

  it('tells archives in a mod root from archives in Paks', () => {
    const base = { modRef: 'a:1', sizeBytes: 10 }
    expect(archiveSubject({ ...base, relPath: 'Mods/skin.zip' }).kind).toBe('archiveInRoot')
    expect(archiveSubject({ ...base, relPath: 'skin.zip' }).kind).toBe('archiveInPaks')
    expect(archiveSubject({ ...base, relPath: 'Mods/skin.zip' }).target.key).toBe('Mods/skin.zip')
  })

  it('keys a leftover by its relative path', () => {
    const subject = leftoverSubject({ modRef: 'l:1', relPath: 'Mods/_x_tmp_ab' })
    expect(subject.target).toEqual({ modRef: 'l:1', key: 'Mods/_x_tmp_ab' })
    expect(subject.kind).toBe('leftover')
  })
})
