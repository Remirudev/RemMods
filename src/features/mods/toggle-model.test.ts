import { describe, expect, it } from 'vitest'
import { makeModRow, makeSnapshot } from '../../test/fixtures'
import {
  bulkEnable,
  canToggle,
  disableAllCount,
  disabledBetween,
  isOn,
  issueFix,
  optimisticSnapshot,
  rowsToChange,
  toggleTargets,
} from './toggle-model'

const OFF_PAYLOAD = { livePak: 0, offPak: 2, liveCompanion: 0, offCompanion: 1 }

function off(name: string) {
  return makeModRow(name, { state: 'disabled', payload: OFF_PAYLOAD })
}

describe('isOn', () => {
  it('counts enabled and partial rows as on', () => {
    expect(isOn(makeModRow('A'))).toBe(true)
    expect(isOn(makeModRow('A', { state: 'partial' }))).toBe(true)
    expect(isOn(off('A'))).toBe(false)
    expect(isOn(makeModRow('A', { state: 'noPakFiles' }))).toBe(false)
  })

  it('treats a prefixed Logic row as off even while its paks are live', () => {
    const logic = makeModRow('Tool', { kind: 'logic', root: 'LogicMods', logicPrefixed: true })
    expect(isOn(logic)).toBe(false)
    expect(isOn({ ...logic, logicPrefixed: false })).toBe(true)
  })
})

describe('canToggle', () => {
  it('follows toggleBlock', () => {
    expect(canToggle(makeModRow('A'))).toBe(true)
    expect(canToggle(makeModRow('A', { toggleBlock: 'noPakFiles' }))).toBe(false)
  })
})

describe('optimisticSnapshot', () => {
  it('switches the named rows and moves their files, keeping the revision', () => {
    const snapshot = makeSnapshot([makeModRow('A'), makeModRow('B')])
    const next = optimisticSnapshot(snapshot, ['m:a'], false)
    expect(next.revision).toBe(snapshot.revision)
    expect(next.rows[0]?.state).toBe('disabled')
    expect(next.rows[0]?.payload).toEqual({
      livePak: 0,
      offPak: 1,
      liveCompanion: 0,
      offCompanion: 0,
    })
    expect(next.rows[1]).toBe(snapshot.rows[1])
  })

  it('enables a row and flips the name prefix of a Logic row', () => {
    const logic = makeModRow('Tool', {
      kind: 'logic',
      logicPrefixed: true,
      state: 'disabled',
      payload: OFF_PAYLOAD,
    })
    const next = optimisticSnapshot(makeSnapshot([logic]), [logic.modRef], true)
    expect(next.rows[0]?.state).toBe('enabled')
    expect(next.rows[0]?.logicPrefixed).toBe(false)
    expect(next.rows[0]?.payload.offPak).toBe(0)
  })

  it('leaves blocked rows alone', () => {
    const blocked = makeModRow('A', { toggleBlock: 'linked' })
    const next = optimisticSnapshot(makeSnapshot([blocked]), [blocked.modRef], false)
    expect(next.rows[0]).toBe(blocked)
  })
})

describe('bulkEnable and rowsToChange', () => {
  it('turns everything off only when every toggleable row is on', () => {
    expect(bulkEnable([makeModRow('A'), makeModRow('B')])).toBe(false)
    expect(bulkEnable([makeModRow('A'), off('B')])).toBe(true)
    expect(bulkEnable([off('A'), off('B')])).toBe(true)
  })

  it('ignores blocked rows', () => {
    const blockedOff = off('C')
    blockedOff.toggleBlock = 'ambiguous'
    expect(bulkEnable([makeModRow('A'), blockedOff])).toBe(false)
  })

  it('lists the toggleable rows that would change', () => {
    const rows = [makeModRow('A'), off('B'), makeModRow('C', { toggleBlock: 'linked' })]
    expect(rowsToChange(rows, false).map((row) => row.name)).toEqual(['A'])
    expect(rowsToChange(rows, true).map((row) => row.name)).toEqual(['B'])
  })
})

describe('toggleTargets', () => {
  it('carries the ref and the key of every row', () => {
    expect(toggleTargets([makeModRow('Alpha')])).toEqual([{ modRef: 'm:alpha', key: 'alpha' }])
  })
})

describe('disableAllCount and disabledBetween', () => {
  it('counts on rows and disabled-but-loaded Logic rows, never blocked ones', () => {
    const loaded = makeModRow('Tool', { kind: 'logic', logicPrefixed: true })
    const rows = [makeModRow('A'), off('B'), loaded, makeModRow('D', { toggleBlock: 'noPakFiles' })]
    expect(disableAllCount(rows)).toBe(2)
  })

  it('counts rows that went from on to off, matched by key', () => {
    const before = makeSnapshot([makeModRow('A'), makeModRow('B'), off('C')])
    const after = makeSnapshot([off('A'), makeModRow('B'), off('C')])
    expect(disabledBetween(before, after)).toBe(1)
  })
})

describe('issueFix', () => {
  it('maps the two Logic mismatches to a direction', () => {
    const loaded = makeModRow('A', { issues: [{ kind: 'logicDisabledButLoaded' }] })
    const idle = makeModRow('B', { issues: [{ kind: 'logicEnabledNotLoaded' }] })
    expect(issueFix(loaded)).toBe(false)
    expect(issueFix(idle)).toBe(true)
    expect(issueFix(makeModRow('C'))).toBeNull()
    expect(issueFix({ ...loaded, toggleBlock: 'linked' })).toBeNull()
  })
})
