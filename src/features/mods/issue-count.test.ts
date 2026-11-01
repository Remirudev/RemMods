import { describe, expect, it } from 'vitest'
import type { RootInfo } from '../../lib/ipc/types'
import { makeModRow } from '../../test/fixtures'
import { countIssues, rootsWithProblems } from './issue-count'

const ROOT: RootInfo = {
  kind: 'standard',
  name: 'Mods',
  isLink: false,
  caseVariant: false,
  unreadable: null,
}

describe('countIssues', () => {
  it('counts rows with issues, roots with problems and leftovers', () => {
    const rows = [makeModRow('a'), makeModRow('b', { issues: [{ kind: 'linkedMod' }] })]
    const roots: RootInfo[] = [
      ROOT,
      { ...ROOT, name: 'mods', caseVariant: true },
      { ...ROOT, name: '~mods', unreadable: { op: 'read_dir', class: 'accessDenied' } },
    ]
    const leftovers = [{ modRef: 'x:1', relPath: 'Mods/.x' }]
    expect(rootsWithProblems(roots)).toHaveLength(2)
    expect(countIssues(rows, roots, leftovers)).toBe(4)
  })

  it('is zero for a clean scan', () => {
    expect(countIssues([makeModRow('a')], [ROOT], [])).toBe(0)
  })
})
