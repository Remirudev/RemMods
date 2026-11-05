import { describe, expect, it } from 'vitest'
import { makeModRow } from '../../test/fixtures'
import { bulkSelection, modActions, selectedRows, spaceAction } from './mod-actions'

const ON = makeModRow('On')
const OFF = makeModRow('Off', { state: 'disabled' })
const BLOCKED = makeModRow('Blocked', { toggleBlock: 'ambiguous' })
const ROWS = [ON, OFF, BLOCKED]
const LINKED = makeModRow('Linked', { toggleBlock: 'linked', issues: [{ kind: 'linkedMod' }] })
const LOOSE = makeModRow('Loose', { kind: 'loose' })

describe('modActions', () => {
  it('counts the rows each action would change', () => {
    expect(modActions(ROWS, ['m:on', 'm:off', 'm:blocked'])).toEqual([
      { id: 'enable', count: 1, disabled: false },
      { id: 'disable', count: 1, disabled: false },
      { id: 'trash', count: 3, disabled: false },
      { id: 'delete', count: 3, disabled: false },
    ])
  })

  it('disables an action with nothing to change', () => {
    expect(modActions(ROWS, ['m:on', 'm:blocked'])).toEqual([
      { id: 'enable', count: 0, disabled: true },
      { id: 'disable', count: 1, disabled: false },
      { id: 'trash', count: 2, disabled: false },
      { id: 'delete', count: 2, disabled: false },
    ])
  })

  it('skips linked rows for trash and delete, and loose rows for delete only', () => {
    const rows = [ON, LINKED, LOOSE]
    const actions = modActions(rows, ['m:on', 'm:linked', 'm:loose'])
    expect(actions.find((action) => action.id === 'trash')).toEqual({
      id: 'trash',
      count: 2,
      disabled: false,
    })
    expect(actions.find((action) => action.id === 'delete')).toEqual({
      id: 'delete',
      count: 1,
      disabled: false,
    })
    const blocked = modActions(rows, ['m:linked', 'm:loose'])
    expect(blocked.find((action) => action.id === 'delete')?.disabled).toBe(true)
  })
})

describe('bulkSelection', () => {
  it('passes only the rows the label counts and the rest as skipped', () => {
    const rows = selectedRows([ON, LINKED, LOOSE, OFF], ['m:on', 'm:linked', 'm:loose', 'm:off'])
    const remove = bulkSelection(rows, 'delete')
    expect(remove.subjects.map((subject) => subject.target.modRef)).toEqual(['m:on', 'm:off'])
    expect(remove.skipped).toBe(2)
    const trash = bulkSelection(rows, 'trash')
    expect(trash.subjects.map((subject) => subject.target.modRef)).toEqual([
      'm:on',
      'm:loose',
      'm:off',
    ])
    expect(trash.skipped).toBe(1)
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
