import { isIpcError } from '../../lib/ipc/ipc-error'

export function isScanNotFound(error: unknown): boolean {
  if (!isIpcError(error)) return false
  const appError = error.appError
  return appError.kind === 'io' && appError.op === 'scan' && appError.class === 'notFound'
}
