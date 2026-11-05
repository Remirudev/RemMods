import { describe, expect, it } from 'vitest'
import { rememberPlatformOs } from '../../shared/trash-place'
import type { AppError } from '../ipc/types'
import { describeAppError } from './error-messages'
import { i18n } from './index'

const ERRORS: AppError[] = [
  { kind: 'busy' },
  { kind: 'io', op: 'scan', class: 'notFound' },
  { kind: 'invalidInput', field: 'modRef' },
  { kind: 'readOnly' },
  { kind: 'internal' },
  { kind: 'stale' },
  { kind: 'gameNotSet' },
  { kind: 'invalidGameFolder', reason: 'notUnicode' },
  { kind: 'launchFailed' },
  { kind: 'revealFailed', reason: 'unsupportedName' },
  { kind: 'io', op: 'probe', class: 'inUse' },
  { kind: 'io', op: 'rename', class: 'alreadyExists' },
  { kind: 'gameRunning' },
  { kind: 'targetExists' },
  { kind: 'outOfScope' },
  { kind: 'toggleBlocked', reason: 'ambiguous' },
  { kind: 'toggleBlocked', reason: 'linked' },
  { kind: 'toggleBlocked', reason: 'unknownState' },
  { kind: 'toggleBlocked', reason: 'noPakFiles' },
  { kind: 'toggleBlocked', reason: 'invalidLogicName' },
  { kind: 'batchFailed', class: 'accessDenied', stuck: 0, folder: false },
  { kind: 'batchFailed', class: 'inUse', stuck: 0, folder: true },
  { kind: 'batchFailed', class: 'other', stuck: 2, folder: false },
  { kind: 'invalidName', issue: 'reserved_device_name' },
  { kind: 'invalidName', issue: 'something_new' },
  { kind: 'manageBlocked', action: 'rename', reason: 'loose' },
  { kind: 'manageBlocked', action: 'trash', reason: 'outsideRoots' },
  { kind: 'manageBlocked', action: 'delete', reason: 'journalPending' },
  { kind: 'trashUnavailable', reason: 'otherDevice', size: 10 },
  { kind: 'trashUnavailable', reason: 'noUsableTrash', size: null },
  { kind: 'trashUnavailable', reason: 'notSupported', size: null },
  { kind: 'trashUnavailable', reason: 'recycleRefused', size: 10 },
  { kind: 'trashUnavailable', reason: 'notShellSafe', size: null },
  { kind: 'busyCancelled' },
  { kind: 'deleteTokenInvalid' },
  { kind: 'modinfoAmbiguous' },
  { kind: 'profileNotFound' },
  { kind: 'profileNameTaken' },
  { kind: 'importFailed', problem: 'tooLarge' },
  { kind: 'importFailed', problem: 'notJson' },
  { kind: 'importFailed', problem: 'wrongShape' },
  { kind: 'importFailed', problem: 'wrongFormat' },
  { kind: 'importFailed', problem: 'newerVersion' },
  { kind: 'archiveFailed', reason: 'multiVolume' },
  { kind: 'archiveFailed', reason: 'encrypted' },
  { kind: 'archiveFailed', reason: 'memoryLimit' },
  { kind: 'installBlocked', reason: 'targetChanged' },
  { kind: 'installBusy' },
  { kind: 'installCancelled' },
  { kind: 'noSpace', needed: 3 * 1024 * 1024, available: 1024 },
  { kind: 'sourceRejected', reason: 'missing' },
  { kind: 'pictureRefused', reason: 'notAnImage' },
  { kind: 'updateNotConfigured' },
  { kind: 'updateNotAllowed', install: 'deb' },
  { kind: 'updateNotAllowed', install: 'unknown' },
  { kind: 'updateBlocked', reason: 'installRunning' },
  { kind: 'updateBlocked', reason: 'journalPending' },
  { kind: 'updateBlocked', reason: 'busy' },
  { kind: 'updateFailed', stage: 'check' },
  { kind: 'updateFailed', stage: 'verify' },
  { kind: 'unsupported', feature: 'graphicsFallback' },
]

describe('describeAppError', () => {
  it.each(['en', 'de'])('translates every kind in %s without placeholders', async (language) => {
    await i18n.changeLanguage(language)
    for (const error of ERRORS) {
      const { title, message } = describeAppError(error)
      expect(title, error.kind).not.toBe('')
      expect(message, error.kind).not.toBe('')
      expect(`${title}${message}`, error.kind).not.toMatch(/\{\{|titles\.|messages\./)
    }
    await i18n.changeLanguage('en')
  })

  it('names both sizes of a full drive', () => {
    const message = describeAppError({
      kind: 'noSpace',
      needed: 3 * 1024 * 1024,
      available: 1024,
    }).message
    expect(message).toContain('3 MB')
    expect(message).toContain('1 KB')
  })

  it('names the folder problem and the reveal problem', () => {
    expect(describeAppError({ kind: 'invalidGameFolder', reason: 'ambiguous' }).message).toContain(
      'differ only in case',
    )
    expect(describeAppError({ kind: 'revealFailed', reason: 'failed' }).message).toContain(
      'No file manager',
    )
  })

  it('tells the three batch failures apart', () => {
    const undone = describeAppError({
      kind: 'batchFailed',
      class: 'accessDenied',
      stuck: 0,
      folder: false,
    })
    expect(undone.message).toContain('access denied')
    expect(undone.message).toContain('undone')
    const folder = describeAppError({ kind: 'batchFailed', class: 'inUse', stuck: 0, folder: true })
    expect(folder.message).toContain('A folder is in use')
    const stuck = describeAppError({ kind: 'batchFailed', class: 'other', stuck: 1, folder: false })
    expect(stuck.message).toContain('1 rename could not be undone')
  })

  it('names the probe failure and the blocked reasons', () => {
    expect(describeAppError({ kind: 'io', op: 'probe', class: 'inUse' }).message).toBe(
      'A mod file is in use. Close TEKKEN 8 first.',
    )
    expect(describeAppError({ kind: 'io', op: 'scan', class: 'inUse' }).message).toContain(
      'another program',
    )
    expect(describeAppError({ kind: 'toggleBlocked', reason: 'noPakFiles' }).message).toBe(
      'This mod has no pak files.',
    )
  })

  it('says a cancelled wait changed nothing', () => {
    expect(describeAppError({ kind: 'busyCancelled' }).message).toBe(
      'Cancelled. No mod was changed.',
    )
  })

  it('names the Recycle Bin on Windows in the trash title', () => {
    const error: AppError = { kind: 'trashUnavailable', reason: 'recycleRefused', size: null }
    rememberPlatformOs('windows')
    expect(describeAppError(error).title).toBe('The Recycle Bin cannot be used')
    rememberPlatformOs('linux')
    expect(describeAppError(error).title).toBe('The Trash cannot be used')
    rememberPlatformOs(null)
  })
})
