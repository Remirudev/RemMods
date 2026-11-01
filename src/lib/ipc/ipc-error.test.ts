import { describe, expect, it } from 'vitest'
import { IpcError, isAppError, unwrapResult } from './ipc-error'
import type { AppError, CommandResult } from './types'

function ok<T>(data: T): Promise<CommandResult<T>> {
  const result: CommandResult<T> = { status: 'ok', data }
  return Promise.resolve(result)
}

function failed(error: AppError): Promise<CommandResult<never>> {
  const result: CommandResult<never> = { status: 'error', error }
  return Promise.resolve(result)
}

async function captureError(pending: Promise<unknown>): Promise<IpcError> {
  try {
    await pending
  } catch (error) {
    if (error instanceof IpcError) return error
    throw error
  }
  throw new Error('expected the call to fail')
}

describe('unwrapResult', () => {
  it('returns the data of an ok result', async () => {
    await expect(unwrapResult('settings_get', () => ok(42))).resolves.toBe(42)
  })

  it('throws an IpcError carrying the typed AppError', async () => {
    const appError: AppError = { kind: 'io', op: 'write', class: 'accessDenied' }
    const error = await captureError(unwrapResult('settings_update', () => failed(appError)))
    expect(error.appError).toEqual(appError)
    expect(error.command).toBe('settings_update')
    expect(error.detail).toBeNull()
  })

  // tauri-specta's typedError turns every non-Error rejection, an ACL denial string included, into
  // an error result, so the payload is not always an AppError.
  it('maps an untyped error result, such as an ACL denial string, to internal', async () => {
    const result = { status: 'error', error: 'open_logs not allowed by ACL' }
    const denied = () => Promise.resolve(result as unknown as CommandResult<null>)
    const error = await captureError(unwrapResult('open_logs', denied))
    expect(error.appError).toEqual({ kind: 'internal' })
    expect(error.detail).toBe('open_logs not allowed by ACL')
  })

  it('maps an untyped rejection to internal', async () => {
    const denied = (): Promise<CommandResult<null>> => Promise.reject('command not allowed')
    const error = await captureError(unwrapResult('open_logs', denied))
    expect(error.appError).toEqual({ kind: 'internal' })
    expect(error.detail).toBe('command not allowed')
  })

  it('maps a thrown Error to internal and keeps its message for the log', async () => {
    const broken = (): Promise<CommandResult<null>> => Promise.reject(new Error('ipc closed'))
    const error = await captureError(unwrapResult('app_ready', broken))
    expect(error.appError).toEqual({ kind: 'internal' })
    expect(error.detail).toBe('ipc closed')
  })
})

describe('isAppError', () => {
  it('accepts every known kind and rejects anything else', () => {
    expect(isAppError({ kind: 'busy' })).toBe(true)
    expect(isAppError({ kind: 'readOnly' })).toBe(true)
    expect(isAppError({ kind: 'surprise' })).toBe(false)
    expect(isAppError('busy')).toBe(false)
    expect(isAppError(null)).toBe(false)
  })
})

describe('isAppError kinds', () => {
  it('knows the kinds added with the game and mods commands', () => {
    for (const kind of [
      'stale',
      'gameNotSet',
      'invalidGameFolder',
      'launchFailed',
      'revealFailed',
    ]) {
      expect(isAppError({ kind }), kind).toBe(true)
    }
  })

  it('rejects unknown kinds and inherited property names', () => {
    expect(isAppError({ kind: 'somethingNew' })).toBe(false)
    expect(isAppError({ kind: 'constructor' })).toBe(false)
    expect(isAppError('stale')).toBe(false)
  })
})
