import { isIpcError } from '../ipc/ipc-error'
import type { AppError } from '../ipc/types'
import { i18n } from './index'
import type { ErrorDescription } from './types'

export function describeAppError(error: AppError): ErrorDescription {
  const t = i18n.getFixedT(null, 'errors')
  switch (error.kind) {
    case 'busy':
      return { title: t(($) => $.titles.busy), message: t(($) => $.messages.busy) }
    case 'io': {
      const ioClass = error.class
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
  }
}

export function describeError(error: unknown): ErrorDescription {
  if (isIpcError(error)) return describeAppError(error.appError)
  const t = i18n.getFixedT(null, 'errors')
  return { title: t(($) => $.titles.unknown), message: t(($) => $.messages.unknown) }
}
