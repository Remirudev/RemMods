import { describe, expect, it } from 'vitest'
import type { RepairAction, RepairItem } from '../../lib/ipc/types'
import { isActionAvailable, planRepairAction } from './repair-model'

function item(overrides: Partial<RepairItem> = {}): RepairItem {
  return {
    id: 'invalidName|m:a',
    kind: 'invalidName',
    target: { modRef: 'm:a', key: 'a' },
    label: 'A',
    actions: [],
    suggestedName: 'A_',
    ...overrides,
  }
}

describe('planRepairAction', () => {
  it('maps every action to an existing command', () => {
    const plans: [RepairAction, unknown][] = [
      ['rename', { kind: 'rename', suggestedName: 'A_' }],
      ['trash', { kind: 'trash' }],
      ['deletePermanently', { kind: 'delete' }],
      ['reveal', { kind: 'reveal' }],
      ['finishDisable', { kind: 'toggle', enable: false }],
      ['finishEnable', { kind: 'toggle', enable: true }],
      ['openLogs', { kind: 'logs' }],
    ]
    for (const [action, plan] of plans) {
      expect(planRepairAction(item(), action), action).toEqual(plan)
    }
  })

  it('carries no suggestion when the core made none', () => {
    expect(planRepairAction(item({ suggestedName: null }), 'rename')).toEqual({
      kind: 'rename',
      suggestedName: null,
    })
  })
})

describe('isActionAvailable', () => {
  it('lets the logs open without a target and nothing else', () => {
    const stuck = item({ kind: 'stuckJournal', target: null })
    expect(isActionAvailable(stuck, 'openLogs')).toBe(true)
    expect(isActionAvailable(stuck, 'trash')).toBe(false)
    expect(isActionAvailable(item(), 'trash')).toBe(true)
  })
})
