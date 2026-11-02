import type { AppError, CommandResult } from './types'

const KNOWN_KINDS: Record<AppError['kind'], true> = {
  busy: true,
  io: true,
  invalidInput: true,
  readOnly: true,
  internal: true,
  stale: true,
  gameNotSet: true,
  invalidGameFolder: true,
  launchFailed: true,
  revealFailed: true,
  gameRunning: true,
  targetExists: true,
  outOfScope: true,
  toggleBlocked: true,
  batchFailed: true,
  invalidName: true,
  manageBlocked: true,
  trashUnavailable: true,
  deleteTokenInvalid: true,
  modinfoAmbiguous: true,
  profileNotFound: true,
  profileNameTaken: true,
  importFailed: true,
  archiveFailed: true,
  installBlocked: true,
  installBusy: true,
  installCancelled: true,
  noSpace: true,
  sourceRejected: true,
  updateNotConfigured: true,
  updateNotAllowed: true,
  updateBlocked: true,
  updateFailed: true,
  unsupported: true,
  busyCancelled: true,
}

export class IpcError extends Error {
  readonly command: string
  readonly appError: AppError
  readonly detail: string | null

  constructor(command: string, appError: AppError, detail: string | null = null) {
    super(`${command} failed: ${appError.kind}`)
    this.name = 'IpcError'
    this.command = command
    this.appError = appError
    this.detail = detail
  }
}

export function isAppError(value: unknown): value is AppError {
  if (typeof value !== 'object' || value === null || !('kind' in value)) return false
  return typeof value.kind === 'string' && Object.hasOwn(KNOWN_KINDS, value.kind)
}

export function isIpcError(value: unknown): value is IpcError {
  return value instanceof IpcError
}

function describeReason(reason: unknown): string {
  if (reason instanceof Error) return reason.message
  if (typeof reason !== 'object' || reason === null) return String(reason)
  try {
    return JSON.stringify(reason)
  } catch {
    return String(reason)
  }
}

export function describeForLog(error: unknown): string {
  if (!isIpcError(error)) return describeReason(error)
  return error.detail ? `${error.message} (${error.detail})` : error.message
}

// Tauri rejects with a plain string for transport and ACL failures, so anything that is not a
// typed AppError becomes `internal` and keeps the raw reason for the log.
export function toIpcError(command: string, reason: unknown): IpcError {
  if (reason instanceof IpcError) return reason
  if (isAppError(reason)) return new IpcError(command, reason)
  return new IpcError(command, { kind: 'internal' }, describeReason(reason))
}

export async function unwrapResult<T>(
  command: string,
  invoke: () => Promise<CommandResult<T>>,
): Promise<T> {
  let result: CommandResult<T>
  try {
    result = await invoke()
  } catch (reason) {
    throw toIpcError(command, reason)
  }
  if (result.status === 'error') throw toIpcError(command, result.error)
  return result.data
}
