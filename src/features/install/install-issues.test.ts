import { describe, expect, it } from 'vitest'
import { enResources } from '../../lib/i18n/resources'
import { makeIssue } from '../../test/install-fixtures'
import { ALL_ISSUE_CODES, groupIssues, issueTextKey } from './install-issues'

describe('groupIssues', () => {
  it('orders blocks, warnings and notes and keeps the backend order inside a group', () => {
    const groups = groupIssues([
      makeIssue('link', 'info'),
      makeIssue('invalidName', 'warn', { path: 'a' }),
      makeIssue('noPayload', 'block'),
      makeIssue('replaces', 'warn', { path: 'b' }),
    ])
    expect(groups.map((group) => group.severity)).toEqual(['block', 'warn', 'info'])
    expect(groups[1]?.issues.map((issue) => issue.code)).toEqual(['invalidName', 'replaces'])
  })

  it('leaves out severities without issues', () => {
    expect(groupIssues([makeIssue('link', 'info')]).map((group) => group.severity)).toEqual([
      'info',
    ])
    expect(groupIssues([])).toEqual([])
  })
})

describe('issueTextKey', () => {
  it('has an English text for every code', () => {
    expect(ALL_ISSUE_CODES).toHaveLength(26)
    for (const code of ALL_ISSUE_CODES) {
      const text = enResources.install.issues[issueTextKey(code)]
      expect(text, code).toEqual(expect.any(String))
    }
  })
})
