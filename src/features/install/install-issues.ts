import type { InstallIssue, InstallIssueCode, IssueSeverity } from '../../lib/ipc/types'
import type { IssueGroup, IssueTextKey } from './types'

export const SEVERITY_ORDER: readonly IssueSeverity[] = ['block', 'warn', 'info']

// Typed as a record so a new code from the backend fails the build until it has a text.
const CODES: Record<InstallIssueCode, true> = {
  unsafePath: true,
  link: true,
  special: true,
  antiItem: true,
  duplicateIdentical: true,
  duplicateDiffers: true,
  duplicateRaw: true,
  caseClash: true,
  invalidName: true,
  notPortable: true,
  undecodable: true,
  legacyNames: true,
  nestedArchive: true,
  shipsDisabled: true,
  shipsBothStates: true,
  scriptMod: true,
  noPayload: true,
  mixedRoots: true,
  outsideRoot: true,
  extraFiles: true,
  pathTooLong: true,
  alsoInRoot: true,
  ambiguous: true,
  replaces: true,
  noSpace: true,
  emptyDirs: true,
}

export const ALL_ISSUE_CODES = Object.keys(CODES) as InstallIssueCode[]

export function issueTextKey(code: InstallIssueCode): IssueTextKey {
  return code
}

// Blocks first, then warnings, then notes; the backend's order is kept inside a severity.
export function groupIssues(issues: readonly InstallIssue[]): IssueGroup[] {
  return SEVERITY_ORDER.map((severity) => ({
    severity,
    issues: issues.filter((issue) => issue.severity === severity),
  })).filter((group) => group.issues.length > 0)
}

export function hasIssue(
  issues: readonly InstallIssue[],
  code: InstallIssueCode,
  severity?: IssueSeverity,
): boolean {
  return issues.some((issue) => issue.code === code && (!severity || issue.severity === severity))
}
