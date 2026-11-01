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
})
