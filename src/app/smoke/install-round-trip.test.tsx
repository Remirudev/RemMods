import { describe, expect, it } from 'vitest'
import type { InstallProgress, ModsSnapshot } from '../../lib/ipc/types'
import { makeModRow, makeSnapshot } from '../../test/fixtures'
import {
  makeCandidate,
  makeInspection,
  makeOutcome,
  PROGRESS_EXTRACTING,
} from '../../test/install-fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { findSmokeArchive, runInstallRoundTrip, SMOKE_INSTALL_KEY } from './install-round-trip'
import { checkInstallRoundTrip } from './smoke-checks'

const WITH_ARCHIVE: ModsSnapshot = makeSnapshot([makeModRow('Alpha')], {
  archives: [{ modRef: 'a:smoke', relPath: 'Mods/install-smoke.zip', sizeBytes: 100 }],
})
const WITHOUT_ARCHIVE = makeSnapshot([makeModRow('Alpha')])

function installedSnapshot(): ModsSnapshot {
  const outcome = makeOutcome(['SmokeInstall'])
  return {
    ...outcome.snapshot,
    rows: outcome.snapshot.rows.map((row) => ({ ...row, key: SMOKE_INSTALL_KEY })),
  }
}

describe('runInstallRoundTrip', () => {
  it('is skipped without the fixture archive row', async () => {
    const ipc = installIpcMock()
    expect(await runInstallRoundTrip(WITHOUT_ARCHIVE)).toBeNull()
    expect(ipc.callsOf('install_inspect')).toHaveLength(0)
    expect(checkInstallRoundTrip(null)).toMatchObject({ ok: true, detail: 'no fixture' })
  })

  it('finds the archive by its file name in any folder', () => {
    expect(findSmokeArchive(WITH_ARCHIVE)?.modRef).toBe('a:smoke')
  })

  it('inspects the archive row, installs it and expects the new row', async () => {
    const ipc = installIpcMock(undefined, {
      handlers: {
        mods_refresh: () => WITH_ARCHIVE,
        install_inspect: () => makeInspection([makeCandidate(0, 'SmokeInstall')]),
        install_start: (payload) => {
          const { progress } = payload as { progress: { onmessage: (m: InstallProgress) => void } }
          progress.onmessage(PROGRESS_EXTRACTING)
          return { ...makeOutcome(['SmokeInstall']), snapshot: installedSnapshot() }
        },
      },
    })
    const failures = await runInstallRoundTrip(WITH_ARCHIVE)
    expect(failures).toEqual([])
    expect(checkInstallRoundTrip(failures).ok).toBe(true)
    expect(ipc.callsOf('install_inspect')[0]?.payload).toEqual({
      request: {
        source: { kind: 'archive', row: 'a:smoke' },
        encoding: 'auto',
        root: null,
        names: [],
      },
    })
  })

  it('fails when the new row is missing', async () => {
    installIpcMock(undefined, {
      handlers: {
        mods_refresh: () => WITH_ARCHIVE,
        install_inspect: () => makeInspection([makeCandidate(0, 'SmokeInstall')]),
        install_start: (payload) => {
          const { progress } = payload as { progress: { onmessage: (m: InstallProgress) => void } }
          progress.onmessage(PROGRESS_EXTRACTING)
          return makeOutcome(['Other'])
        },
      },
    })
    const failures = await runInstallRoundTrip(WITH_ARCHIVE)
    expect(failures?.join('\n')).toContain('row missing after the install')
    expect(checkInstallRoundTrip(failures).ok).toBe(false)
  })

  it('fails on a blocked inspection without starting', async () => {
    const ipc = installIpcMock(undefined, {
      handlers: {
        mods_refresh: () => WITH_ARCHIVE,
        install_inspect: () =>
          makeInspection([makeCandidate(0, 'SmokeInstall')], { blocked: true }),
      },
    })
    expect(await runInstallRoundTrip(WITH_ARCHIVE)).toContain('inspection is blocked')
    expect(ipc.callsOf('install_start')).toHaveLength(0)
  })
})
