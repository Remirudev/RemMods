import { describe, expect, it } from 'vitest'
import { i18n } from '../../lib/i18n'
import type { ModIssue } from '../../lib/ipc/types'
import { issueMessage } from './issue-message'

const t = i18n.getFixedT('en', 'mods')
const tErrors = i18n.getFixedT('en', 'errors')

function message(issue: ModIssue): string {
  return issueMessage(issue, 'Mods', t, tErrors, 'en')
}

describe('issueMessage', () => {
  it('writes a sentence for every kind, without leftover placeholders or keys', () => {
    const issues: ModIssue[] = [
      { kind: 'ambiguousKey' },
      { kind: 'caseCollision', others: ['A', 'B'] },
      { kind: 'caseVariantRoot' },
      { kind: 'nonUnicodeName' },
      { kind: 'invalidName', problem: 'forbiddenChar' },
      { kind: 'nameNotPortable', problem: 'tooLong' },
      { kind: 'disabledPrefixName' },
      { kind: 'emptyName' },
      { kind: 'logicDisabledButLoaded' },
      { kind: 'logicEnabledNotLoaded' },
      { kind: 'modinfoInvalid' },
      { kind: 'modinfoCaseVariants' },
      { kind: 'pathTooLong', longest: 280 },
      { kind: 'linkedRoot' },
      { kind: 'linkedMod' },
      { kind: 'containsLink' },
      { kind: 'looseCopy', files: ['a.pak'] },
      { kind: 'looseCaseVariants' },
      { kind: 'nonUnicodeFiles', count: 2 },
      { kind: 'invalidFileNames', count: 1 },
      { kind: 'nonPortableFileNames', count: 3 },
      { kind: 'unreadable', problem: { op: 'read_dir', class: 'accessDenied' } },
    ]
    for (const issue of issues) {
      const text = message(issue)
      expect(text, issue.kind).not.toBe('')
      expect(text, issue.kind).not.toContain('{{')
      expect(text, issue.kind).not.toContain(issue.kind)
    }
  })

  it('interpolates names, counts and problem texts', () => {
    expect(message({ kind: 'caseCollision', others: ['A', 'B'] })).toContain('A and B')
    expect(message({ kind: 'pathTooLong', longest: 280 })).toContain('280')
    expect(message({ kind: 'invalidName', problem: 'trailingDotOrSpace' })).toContain(
      'ends with a dot or a space',
    )
    expect(message({ kind: 'caseVariantRoot' })).toContain('Mods')
    expect(message({ kind: 'nonUnicodeFiles', count: 1 })).toContain('1 file name is')
    expect(message({ kind: 'nonUnicodeFiles', count: 2 })).toContain('2 file names are')
    expect(message({ kind: 'unreadable', problem: { op: 'x', class: 'accessDenied' } })).toContain(
      'Access was denied',
    )
  })
})
