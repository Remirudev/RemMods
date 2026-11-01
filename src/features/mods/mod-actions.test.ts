import { describe, expect, it } from 'vitest'
import { makeModRow } from '../../test/fixtures'
import { modActions, spaceAction } from './mod-actions'

const ON = makeModRow('On')
const OFF = makeModRow('Off', { state: 'disabled' })
const BLOCKED = makeModRow('Blocked', { toggleBlock: 'ambiguous' })
const ROWS = [ON, OFF, BLOCKED]

describe('modActions', () => {
  it('counts the rows each action would change', () => {
    expect(modActions(ROWS, ['m:on', 'm:off', 'm:blocked'])).toEqual([
      { id: 'enable', count: 1, disabled: false },
      { id: 'disable', count: 1, disabled: false },
    ])
  })

  it('disables an action with nothing to change', () => {
    expect(modActions(ROWS, ['m:on', 'm:blocked'])).toEqual([
      { id: 'enable', count: 0, disabled: true },
      { id: 'disable', count: 1, disabled: false },
    ])
  })
})

describe('spaceAction', () => {
  it('toggles the active row alone without a multi-selection', () => {
    expect(spaceAction(ROWS, ['m:on'], 'm:on')).toEqual({ enable: false, rows: [ON] })
    expect(spaceAction(ROWS, [], 'm:off')).toEqual({ enable: true, rows: [OFF] })
  })

  it('does nothing for a blocked row or without an active row', () => {
    expect(spaceAction(ROWS, [], 'm:blocked')).toBeNull()
    expect(spaceAction(ROWS, [], null)).toBeNull()
  })

  it('toggles the whole selection when the active row is in it', () => {
    expect(spaceAction(ROWS, ['m:on', 'm:off', 'm:blocked'], 'm:on')).toEqual({
      enable: true,
      rows: [OFF],
    })
    const twoOn = [ON, makeModRow('Also')]
    expect(spaceAction(twoOn, ['m:on', 'm:also'], 'm:on')?.enable).toBe(false)
  })

  it('toggles the active row alone when it is outside the selection', () => {
    expect(spaceAction(ROWS, ['m:on', 'm:off'], 'm:blocked')).toBeNull()
  })
})
