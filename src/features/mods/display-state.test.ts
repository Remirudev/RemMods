import { describe, expect, it } from 'vitest'
import { makeModRow } from '../../test/fixtures'
import { displayState } from './display-state'

const LIVE = { livePak: 2, offPak: 0, liveCompanion: 0, offCompanion: 0 }
const OFF = { livePak: 0, offPak: 2, liveCompanion: 0, offCompanion: 0 }

describe('displayState', () => {
  it('keeps the state of an ordinary row', () => {
    expect(displayState(makeModRow('a', { state: 'enabled', payload: LIVE }))).toBe('enabled')
    expect(displayState(makeModRow('b', { state: 'unknown' }))).toBe('unknown')
  })

  it('calls a prefixed folder with live paks disabled-but-loaded', () => {
    const row = makeModRow('c', { state: 'disabled', logicPrefixed: true, payload: LIVE })
    expect(displayState(row)).toBe('disabledLoaded')
  })

  it('keeps a prefixed folder without live paks disabled', () => {
    const row = makeModRow('d', { state: 'disabled', logicPrefixed: true, payload: OFF })
    expect(displayState(row)).toBe('disabled')
  })
})
