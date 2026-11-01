import { describe, expect, it } from 'vitest'
import { ISSUE_SEVERITY, worstSeverity } from './issue-severity'
import type { IssueKind } from './types'

const ERRORS: IssueKind[] = ['ambiguousKey', 'emptyName', 'unreadable']
const INFOS: IssueKind[] = ['disabledPrefixName', 'linkedRoot', 'looseCaseVariants']

describe('ISSUE_SEVERITY', () => {
  it('rates every kind: errors, infos, and warnings for the rest', () => {
    for (const [kind, severity] of Object.entries(ISSUE_SEVERITY)) {
      const expected = ERRORS.includes(kind as IssueKind)
        ? 'error'
        : INFOS.includes(kind as IssueKind)
          ? 'info'
          : 'warning'
      expect(severity, kind).toBe(expected)
    }
  })

  it('rates containsLink and the portability issues as warnings', () => {
    expect(ISSUE_SEVERITY.containsLink).toBe('warning')
    expect(ISSUE_SEVERITY.nameNotPortable).toBe('warning')
    expect(ISSUE_SEVERITY.nonPortableFileNames).toBe('warning')
  })
})

describe('worstSeverity', () => {
  it('is null without issues and otherwise the highest severity', () => {
    expect(worstSeverity([])).toBeNull()
    expect(worstSeverity([{ kind: 'linkedRoot' }])).toBe('info')
    expect(worstSeverity([{ kind: 'linkedRoot' }, { kind: 'linkedMod' }])).toBe('warning')
    expect(worstSeverity([{ kind: 'linkedMod' }, { kind: 'emptyName' }])).toBe('error')
  })
})
