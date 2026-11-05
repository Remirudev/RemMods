import type { GameStatus } from '../../lib/ipc/types'
import type { GameStatusEvent } from './types'

// The event carries only what the monitor tracks; the location and candidates stay as cached.
export function applyGameStatusEvent(
  status: GameStatus | undefined,
  event: GameStatusEvent,
): GameStatus | undefined {
  if (!status) return status
  return {
    ...status,
    running: event.running,
    session: event.session,
    otherManager: event.otherManager,
    update: event.update,
  }
}
