import { ipc } from '../../lib/ipc'
import { describeForLog } from '../../lib/ipc/ipc-error'
import type { ModState, ModsSnapshot } from '../../lib/ipc/types'
import { expectRowState, findRowByKey, hasToggleFixture, TOGGLE_FIXTURE_KEYS } from './smoke-checks'

const LEGS: readonly { enable: boolean; expected: ModState }[] = [
  { enable: false, expected: 'disabled' },
  { enable: true, expected: 'enabled' },
]

// Off and on again; a Logic toggle renames the folder, so the ref is read from the latest
// snapshot before each leg. A failed leg stops the walk and leaves its failure behind.
async function walkRow(
  start: ModsSnapshot,
  key: string,
  failures: string[],
): Promise<ModsSnapshot> {
  let snapshot = start
  for (const { enable, expected } of LEGS) {
    const row = findRowByKey(snapshot, key)
    if (!row) {
      failures.push(`${key}: row missing before toggling ${enable ? 'on' : 'off'}`)
      return snapshot
    }
    try {
      const request = { targets: [{ modRef: row.modRef, key }], enable, applyWhileRunning: false }
      snapshot = (await ipc.modsToggle(request)).snapshot
    } catch (error) {
      failures.push(`${key}: ${describeForLog(error)}`)
      return snapshot
    }
    const problem = expectRowState(snapshot, key, expected)
    if (problem) failures.push(problem)
  }
  return snapshot
}

// The failures, or null when the run has no fixture marker rows. The fixture ends as it began,
// so the backend's comparison of the scan with its marker still holds.
export async function runToggleRoundTrip(start: ModsSnapshot): Promise<string[] | null> {
  if (!hasToggleFixture(start)) return null
  const failures: string[] = []
  let snapshot = start
  for (const key of TOGGLE_FIXTURE_KEYS) {
    snapshot = await walkRow(snapshot, key, failures)
  }
  return failures
}
