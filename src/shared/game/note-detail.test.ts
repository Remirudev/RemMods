import { describe, expect, it } from 'vitest'
import { isNoteDetailKey, isSteamKindKey } from './note-detail'

describe('note detail keys', () => {
  it('knows the Steam kinds and the parse reasons', () => {
    expect(isSteamKindKey('flatpak')).toBe(true)
    expect(isSteamKindKey('toString')).toBe(false)
    expect(isNoteDetailKey('unterminatedString')).toBe(true)
    expect(isNoteDetailKey('wrongAppId')).toBe(true)
  })

  it('rejects codes the UI has no text for', () => {
    expect(isNoteDetailKey('somethingNew')).toBe(false)
    expect(isNoteDetailKey('constructor')).toBe(false)
  })
})
