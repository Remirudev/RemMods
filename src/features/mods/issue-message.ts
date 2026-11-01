import type { TFunction } from 'i18next'
import type { ModIssue } from '../../lib/ipc/types'

// Every issue kind has its own sentence; tsc fails here when the backend adds a kind.
export function issueMessage(
  issue: ModIssue,
  root: string,
  t: TFunction<'mods'>,
  tErrors: TFunction<'errors'>,
  locale: string,
): string {
  const list = (items: readonly string[]) =>
    new Intl.ListFormat(locale, { type: 'conjunction' }).format(items)
  switch (issue.kind) {
    case 'ambiguousKey':
      return t(($) => $.issue.ambiguousKey)
    case 'caseCollision':
      return t(($) => $.issue.caseCollision, { others: list(issue.others) })
    case 'caseVariantRoot':
      return t(($) => $.issue.caseVariantRoot, { root })
    case 'nonUnicodeName':
      return t(($) => $.issue.nonUnicodeName)
    case 'invalidName': {
      const problem = issue.problem
      return t(($) => $.issue.invalidName, { problem: t(($) => $.nameProblems[problem]) })
    }
    case 'nameNotPortable': {
      const problem = issue.problem
      return t(($) => $.issue.nameNotPortable, { problem: t(($) => $.nameProblems[problem]) })
    }
    case 'disabledPrefixName':
      return t(($) => $.issue.disabledPrefixName)
    case 'emptyName':
      return t(($) => $.issue.emptyName)
    case 'logicDisabledButLoaded':
      return t(($) => $.issue.logicDisabledButLoaded)
    case 'logicEnabledNotLoaded':
      return t(($) => $.issue.logicEnabledNotLoaded)
    case 'modinfoInvalid':
      return t(($) => $.issue.modinfoInvalid)
    case 'modinfoCaseVariants':
      return t(($) => $.issue.modinfoCaseVariants)
    case 'pathTooLong':
      return t(($) => $.issue.pathTooLong, { longest: issue.longest })
    case 'linkedRoot':
      return t(($) => $.issue.linkedRoot)
    case 'linkedMod':
      return t(($) => $.issue.linkedMod)
    case 'containsLink':
      return t(($) => $.issue.containsLink)
    case 'looseCopy':
      return t(($) => $.issue.looseCopy, { files: list(issue.files) })
    case 'looseCaseVariants':
      return t(($) => $.issue.looseCaseVariants)
    case 'nonUnicodeFiles':
      return t(($) => $.issue.nonUnicodeFiles, { count: issue.count })
    case 'invalidFileNames':
      return t(($) => $.issue.invalidFileNames, { count: issue.count })
    case 'nonPortableFileNames':
      return t(($) => $.issue.nonPortableFileNames, { count: issue.count })
    case 'unreadable': {
      const ioClass = issue.problem.class
      return t(($) => $.issue.unreadable, { reason: tErrors(($) => $.ioClasses[ioClass]) })
    }
  }
}
