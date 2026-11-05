import type { Channel } from '@tauri-apps/api/core'
import type {
  Inspection,
  InspectRequest,
  InstallOutcome,
  InstallProgress,
  StartRequest,
} from '../../src/lib/ipc/types'
import { makeModRow } from '../../src/test/fixtures'
import { makeCandidate, makeInspection } from '../../src/test/install-fixtures'
import { changed, fail } from './state'
import type { MockState } from './types'

const INSTALLED = 'Echo'

// The spec ends a running install itself, so the progress view stays until it has checked it.
// Latched: a release that comes before the install waits is not lost.
let release: (() => void) | null = null
let released = false

export function releaseInstall(): void {
  released = true
  release?.()
}

function installGate(): Promise<void> {
  if (released) return Promise.resolve()
  return new Promise((resolve) => {
    release = resolve
  })
}

// Only the archive row in Paks can be inspected; picked or dropped files never exist here.
export function inspect(state: MockState, request: InspectRequest): Inspection {
  const source = request.source
  const archive =
    source.kind === 'archive'
      ? state.archives.find((entry) => entry.modRef === source.row)
      : undefined
  if (!archive) fail({ kind: 'sourceRejected', reason: 'missing' })
  const inspection = makeInspection([makeCandidate(0, INSTALLED)], {
    id: `insp-${String(state.nextId).padStart(32, '0')}`,
    sourceName: archive.relPath.split('/').pop() ?? archive.relPath,
    encoding: request.encoding,
  })
  state.nextId += 1
  state.inspections.push(inspection)
  return inspection
}

function progress(phase: InstallProgress['phase'], doneFiles: number): InstallProgress {
  return { phase, doneBytes: doneFiles * 2048, totalBytes: 4096, doneFiles, totalFiles: 2 }
}

// Two progress messages through the channel, then the new row appears once the spec releases it.
export async function start(
  state: MockState,
  request: StartRequest,
  channel: Channel<InstallProgress>,
): Promise<InstallOutcome> {
  const inspection = state.inspections.find((entry) => entry.id === request.inspection)
  if (!inspection) fail({ kind: 'stale' })
  channel.onmessage(progress('extracting', 1))
  channel.onmessage(progress('committing', 2))
  await installGate()
  released = false
  release = null
  const row = makeModRow(INSTALLED)
  state.rows.push(row)
  return {
    snapshot: changed(state),
    installed: [
      { key: row.key, modRef: row.modRef, rel: `Mods/${INSTALLED}`, files: 2, replaced: null },
    ],
  }
}
