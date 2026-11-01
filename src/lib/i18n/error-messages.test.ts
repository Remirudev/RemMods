import { describe, expect, it } from 'vitest'
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
})
