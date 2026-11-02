import { Channel } from '@tauri-apps/api/core'
import { ipc } from '../../lib/ipc'
import { describeForLog } from '../../lib/ipc/ipc-error'
import type { ArchiveEntry, InstallProgress, ModsSnapshot } from '../../lib/ipc/types'

// Only the generated smoke fixture carries this archive; a run without `--paks` has none.
export const SMOKE_ARCHIVE = 'install-smoke.zip'
export const SMOKE_INSTALL_KEY = 'Standard:SmokeInstall'

const PROGRESS_WAIT_MS = 1_000
const POLL_MS = 25

function baseName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path
}

export function findSmokeArchive(snapshot: ModsSnapshot): ArchiveEntry | undefined {
  return snapshot.archives.find((archive) => baseName(archive.relPath) === SMOKE_ARCHIVE)
}

// Channel messages are delivered as events, so the last one can trail the command's answer.
async function waitForMessages(messages: readonly unknown[]): Promise<void> {
  const deadline = Date.now() + PROGRESS_WAIT_MS
  while (messages.length === 0 && Date.now() < deadline) {
    await new Promise((resolve) => window.setTimeout(resolve, POLL_MS))
  }
}

// Inspects the fixture archive and installs its single mod: the failures, or null when the
// snapshot has no such archive.
export async function runInstallRoundTrip(start: ModsSnapshot): Promise<string[] | null> {
  if (!findSmokeArchive(start)) return null
  const failures: string[] = []
  try {
    // The toggle round trip changed the disk since `start`, so the archive row is read again.
    const archive = findSmokeArchive(await ipc.modsRefresh())
    if (!archive) return [`${SMOKE_ARCHIVE}: row missing after the toggle round trip`]
    const inspection = await ipc.installInspect({
      source: { kind: 'archive', row: archive.modRef },
      encoding: 'auto',
      root: null,
      names: [],
    })
    const candidate = inspection.candidates[0]
    if (inspection.blocked) failures.push('inspection is blocked')
    if (inspection.candidates.length !== 1) {
      failures.push(`${inspection.candidates.length} candidates, expected 1`)
    }
    if (!candidate || failures.length > 0) return failures

    const messages: InstallProgress[] = []
    const channel = new Channel<InstallProgress>()
    channel.onmessage = (message) => messages.push(message)
    const outcome = await ipc.installStart(
      {
        inspection: inspection.id,
        picks: [{ id: candidate.id, replace: false }],
        clashes: [],
        autoFix: true,
        applyWhileRunning: false,
      },
      channel,
    )
    await waitForMessages(messages)
    if (messages.length === 0) failures.push('no progress message arrived')
    if (!outcome.snapshot.rows.some((row) => row.key === SMOKE_INSTALL_KEY)) {
      failures.push(`${SMOKE_INSTALL_KEY}: row missing after the install`)
    }
  } catch (error) {
    failures.push(describeForLog(error))
  }
  return failures
}
