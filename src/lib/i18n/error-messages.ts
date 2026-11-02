import { formatSize } from '../../shared/format-size'
import { trashPlaceWord } from '../../shared/trash-place'
import { isIpcError } from '../ipc/ipc-error'
import type { AppError } from '../ipc/types'
import { i18n } from './index'
import type { ErrorDescription } from './types'

const NAME_ISSUES = [
  'empty',
  'dot_name',
  'too_long',
  'forbidden_char',
  'control_char',
  'bidi_control',
  'trailing_dot_or_space',
  'reserved_device_name',
  'disabled_prefix',
  'temp_name',
] as const

export function isNameIssue(value: string): value is (typeof NAME_ISSUES)[number] {
  return (NAME_ISSUES as readonly string[]).includes(value)
}

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
    case 'invalidName': {
      const tMods = i18n.getFixedT(null, 'mods')
      const issue = isNameIssue(error.issue) ? error.issue : 'invalid'
      return {
        title: t(($) => $.titles.invalidName),
        message: tMods(($) => $.nameIssue[issue]),
      }
    }
    case 'manageBlocked': {
      const reason = error.reason
      const action = error.action
      return {
        title: t(($) => $.titles.manageBlocked),
        message: `${t(($) => $.manageBlocked.action[action], { place: trashPlaceWord() })} ${t(($) => $.manageBlocked.reason[reason])}`,
      }
    }
    case 'trashUnavailable': {
      const reason = error.reason
      return {
        title: t(($) => $.titles.trashUnavailable, { place: trashPlaceWord() }),
        message: t(($) => $.trashUnavailable[reason]),
      }
    }
    case 'deleteTokenInvalid':
      return {
        title: t(($) => $.titles.deleteTokenInvalid),
        message: t(($) => $.messages.deleteTokenInvalid),
      }
    case 'modinfoAmbiguous':
      return {
        title: t(($) => $.titles.modinfoAmbiguous),
        message: t(($) => $.messages.modinfoAmbiguous),
      }
    case 'profileNotFound':
      return {
        title: t(($) => $.titles.profileNotFound),
        message: t(($) => $.messages.profileNotFound),
      }
    case 'profileNameTaken':
      return {
        title: t(($) => $.titles.profileNameTaken),
        message: t(($) => $.messages.profileNameTaken),
      }
    case 'importFailed': {
      const problem = error.problem
      return {
        title: t(($) => $.titles.importFailed),
        message: t(($) => $.importFailed[problem]),
      }
    }
    case 'archiveFailed': {
      const reason = error.reason
      return {
        title: t(($) => $.titles.archiveFailed),
        message: t(($) => $.archiveFailed[reason]),
      }
    }
    case 'installBlocked': {
      const reason = error.reason
      return {
        title: t(($) => $.titles.installBlocked),
        message: t(($) => $.installBlocked[reason]),
      }
    }
    case 'installBusy':
      return {
        title: t(($) => $.titles.installBusy),
        message: t(($) => $.messages.installBusy),
      }
    case 'installCancelled':
      return {
        title: t(($) => $.titles.installCancelled),
        message: t(($) => $.messages.installCancelled),
      }
    case 'noSpace': {
      const needed = formatSize(error.needed, i18n.language) ?? ''
      const available = formatSize(error.available, i18n.language) ?? ''
      return {
        title: t(($) => $.titles.noSpace),
        message: t(($) => $.messages.noSpace, { needed, available }),
      }
    }
    case 'sourceRejected': {
      const reason = error.reason
      return {
        title: t(($) => $.titles.sourceRejected),
        message: t(($) => $.sourceRejected[reason]),
      }
    }
    case 'updateNotConfigured':
      return {
        title: t(($) => $.titles.updateNotConfigured),
        message: t(($) => $.messages.updateNotConfigured),
      }
    case 'updateNotAllowed': {
      const install = error.install
      return {
        title: t(($) => $.titles.updateNotAllowed),
        message: t(($) => $.updateNotAllowed[install]),
      }
    }
    case 'updateBlocked': {
      const reason = error.reason
      return {
        title: t(($) => $.titles.updateBlocked),
        message: t(($) => $.updateBlocked[reason]),
      }
    }
    case 'updateFailed': {
      const stage = error.stage
      return {
        title: t(($) => $.titles.updateFailed),
        message: t(($) => $.updateFailed[stage]),
      }
    }
    case 'unsupported': {
      const feature = error.feature
      return {
        title: t(($) => $.titles.unsupported),
        message: t(($) => $.unsupported[feature]),
      }
    }
    case 'busyCancelled':
      return {
        title: t(($) => $.titles.busyCancelled),
        message: t(($) => $.messages.busyCancelled),
      }
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
