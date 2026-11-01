import type { ModRow } from '../../lib/ipc/types'
import type { ModDisplayState } from './types'

// A prefixed LogicMods folder with live paks is disabled by its name only: the game still loads
// it, so neither the badge nor the `disabled` filter may call it disabled.
export function displayState(row: ModRow): ModDisplayState {
  return row.logicPrefixed && row.payload.livePak > 0 ? 'disabledLoaded' : row.state
}
