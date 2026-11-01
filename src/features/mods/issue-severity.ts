import type { ModIssue } from '../../lib/ipc/types'
import type { IssueKind, IssueSeverity } from './types'

export const ISSUE_SEVERITY: Record<IssueKind, IssueSeverity> = {
  ambiguousKey: 'error',
  caseCollision: 'warning',
  caseVariantRoot: 'warning',
  nonUnicodeName: 'warning',
  invalidName: 'warning',
  nameNotPortable: 'warning',
  disabledPrefixName: 'info',
  emptyName: 'error',
  logicDisabledButLoaded: 'warning',
  logicEnabledNotLoaded: 'warning',
  modinfoInvalid: 'warning',
  modinfoCaseVariants: 'warning',
  pathTooLong: 'warning',
  linkedRoot: 'info',
  linkedMod: 'warning',
  containsLink: 'warning',
  looseCopy: 'warning',
  looseCaseVariants: 'info',
  nonUnicodeFiles: 'warning',
  invalidFileNames: 'warning',
  nonPortableFileNames: 'warning',
  unreadable: 'error',
}

const SEVERITY_RANK: Record<IssueSeverity, number> = { info: 0, warning: 1, error: 2 }

export const SEVERITY_COLOR: Record<IssueSeverity, string> = {
  error: 'red',
  warning: 'yellow',
  info: 'blue',
}

export function worstSeverity(issues: readonly ModIssue[]): IssueSeverity | null {
  let worst: IssueSeverity | null = null
  for (const issue of issues) {
    const severity = ISSUE_SEVERITY[issue.kind]
    if (worst === null || SEVERITY_RANK[severity] > SEVERITY_RANK[worst]) worst = severity
  }
  return worst
}
