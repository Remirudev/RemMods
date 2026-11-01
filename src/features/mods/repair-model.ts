import type { RepairAction, RepairItem } from '../../lib/ipc/types'
import type { RepairOperation } from './types'

// There is no repair command: each action is an existing command (SPEC-P3 §5.6).
export function planRepairAction(item: RepairItem, action: RepairAction): RepairOperation {
  switch (action) {
    case 'rename':
      return { kind: 'rename', suggestedName: item.suggestedName }
    case 'trash':
      return { kind: 'trash' }
    case 'deletePermanently':
      return { kind: 'delete' }
    case 'reveal':
      return { kind: 'reveal' }
    case 'finishDisable':
      return { kind: 'toggle', enable: false }
    case 'finishEnable':
      return { kind: 'toggle', enable: true }
    case 'openLogs':
      return { kind: 'logs' }
  }
}

// Actions on a file or folder need the item's target; opening the logs does not.
export function isActionAvailable(item: RepairItem, action: RepairAction): boolean {
  return action === 'openLogs' || item.target !== null
}
