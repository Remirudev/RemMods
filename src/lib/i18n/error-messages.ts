import { isIpcError } from '../ipc/ipc-error'
import type { AppError } from '../ipc/types'
import { i18n } from './index'
import type { ErrorDescription } from './types'

type BatchFailure = Extract<AppError, { kind: 'batchFailed' }>

function describeBatchFailure(error: BatchFailure): string {
  const t = i18n.getFixedT(null, 'errors')
  const ioClass = error.class
  if (error.stuck > 0) return t(($) => $.messages.batchStuck, { count: error.stuck })
  if (error.folder && ioClass === 'inUse') return t(($) => $.messages.batchFolderInUse)
  return t(($) => $.messages.batchFailed, { reason: t(($) => $.ioClassNames[ioClass]) })
}

export function describeAppError(error: AppError): ErrorDescription {
  const t = i18n.getFixedT(null, 'errors')
  switch (error.kind) {
    case 'busy':
      return { title: t(($) => $.titles.busy), message: t(($) => $.messages.busy) }
    case 'io': {
      const ioClass = error.class
      if (error.op === 'probe' && ioClass === 'inUse') {
        return { title: t(($) => $.titles.io), message: t(($) => $.messages.probeInUse) }
      }
      return { title: t(($) => $.titles.io), message: t(($) => $.ioClasses[ioClass]) }
    }
    case 'invalidInput': {
      const field = error.field
      return {
        title: t(($) => $.titles.invalidInput),
        message: t(($) => $.messages.invalidInput, { field }),
      }
    }
    case 'readOnly':
      return { title: t(($) => $.titles.readOnly), message: t(($) => $.messages.readOnly) }
    case 'internal':
      return { title: t(($) => $.titles.internal), message: t(($) => $.messages.internal) }
    case 'stale':
      return { title: t(($) => $.titles.stale), message: t(($) => $.messages.stale) }
    case 'gameNotSet':
      return { title: t(($) => $.titles.gameNotSet), message: t(($) => $.messages.gameNotSet) }
    case 'invalidGameFolder': {
      const reason = error.reason
      return {
        title: t(($) => $.titles.invalidGameFolder),
        message: t(($) => $.folderProblems[reason]),
      }
    }
    case 'launchFailed':
      return { title: t(($) => $.titles.launchFailed), message: t(($) => $.messages.launchFailed) }
    case 'gameRunning':
      return { title: t(($) => $.titles.gameRunning), message: t(($) => $.messages.gameRunning) }
    case 'targetExists':
      return { title: t(($) => $.titles.targetExists), message: t(($) => $.messages.targetExists) }
    case 'outOfScope':
      return { title: t(($) => $.titles.outOfScope), message: t(($) => $.messages.outOfScope) }
    case 'toggleBlocked': {
      const reason = error.reason
      return {
        title: t(($) => $.titles.toggleBlocked),
        message: t(($) => $.toggleBlocked[reason]),
      }
    }
    case 'batchFailed':
      return { title: t(($) => $.titles.batchFailed), message: describeBatchFailure(error) }
    case 'revealFailed': {
      const reason = error.reason
      return {
        title: t(($) => $.titles.revealFailed),
        message: t(($) => $.revealProblems[reason]),
      }
    }
  }
}

export function describeError(error: unknown): ErrorDescription {
  if (isIpcError(error)) return describeAppError(error.appError)
  const t = i18n.getFixedT(null, 'errors')
  return { title: t(($) => $.titles.unknown), message: t(($) => $.messages.unknown) }
}
