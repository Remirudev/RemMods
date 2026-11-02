import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { InstallOutcome, InstallProgress, StartRequest } from '../../lib/ipc/types'
import type { QueuedSource } from '../../shared/types'
import {
  makeCandidate,
  makeInspection,
  makeOutcome,
  PROGRESS_COMMITTING,
  PROGRESS_EXTRACTING,
} from '../../test/install-fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { renderWithProviders } from '../../test/render'
import type { IpcHandler } from '../../test/types'
import { InstallModal } from './InstallModal'

const SOURCE: QueuedSource = { kind: 'token', id: 'src-1', name: 'cool-mod.zip', sourceKind: 'zip' }

interface ProgressChannel {
  onmessage: (message: InstallProgress) => void
}

interface Deferred {
  promise: Promise<InstallOutcome>
  resolve: (outcome: InstallOutcome) => void
  reject: (error: unknown) => void
}

function deferred(): Deferred {
  let resolve: Deferred['resolve'] = () => undefined
  let reject: Deferred['reject'] = () => undefined
  const promise = new Promise<InstallOutcome>((res, rej) => {
    resolve = res
    reject = rej
  })
  return { promise, resolve, reject }
}

function startRequests(ipc: ReturnType<typeof installIpcMock>): StartRequest[] {
  return ipc
    .callsOf('install_start')
    .map((call) => (call.payload as { request: StartRequest }).request)
}

function renderModal(handlers: Record<string, IpcHandler>, remaining = 0) {
  const onClose = vi.fn()
  const onStop = vi.fn()
  const ipc = installIpcMock(undefined, {
    handlers: {
      install_inspect: () => makeInspection([makeCandidate(0, 'Cool')]),
      ...handlers,
    },
  })
  renderWithProviders(
    <InstallModal source={SOURCE} remaining={remaining} onClose={onClose} onStop={onStop} />,
  )
  return { ipc, onClose, onStop }
}

const clickInstall = async (user: ReturnType<typeof userEvent.setup>) =>
  user.click(await screen.findByRole('button', { name: 'Install' }))

describe('InstallModal', () => {
  it('runs the install: progress updates the bar, then the result lists the mod', async () => {
    const user = userEvent.setup()
    const run = deferred()
    const { ipc } = renderModal({
      install_start: (payload) => {
        const { progress } = payload as { progress: ProgressChannel }
        progress.onmessage(PROGRESS_EXTRACTING)
        return run.promise
      },
    })

    await clickInstall(user)
    expect(await screen.findByText('Unpacking…')).toBeVisible()
    expect(screen.getByText('1 / 2 files')).toBeVisible()
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '50')

    run.resolve(makeOutcome(['Cool']))
    expect(await screen.findByText('1 mod installed')).toBeVisible()
    expect(screen.getByText('Mods/Cool')).toBeVisible()
    expect(startRequests(ipc)).toEqual([
      {
        inspection: `insp-${'a'.repeat(32)}`,
        picks: [{ id: 0, replace: false }],
        clashes: [],
        autoFix: true,
        applyWhileRunning: false,
      },
    ])
  })

  it('says whether a replaced mod is in the Trash or kept in Repair', async () => {
    const user = userEvent.setup()
    const outcome = makeOutcome(['Cool'])
    const [mod] = outcome.installed
    if (mod) mod.replaced = { kind: 'keptInRepair', reason: 'trashUnavailable' }
    renderModal({ install_start: () => outcome })
    await clickInstall(user)
    expect(await screen.findByText(/kept in Repair/)).toBeVisible()
  })

  it('cancels through install_cancel and is disabled while committing', async () => {
    const user = userEvent.setup()
    const run = deferred()
    const { ipc } = renderModal({
      install_start: (payload) => {
        const { progress } = payload as { progress: ProgressChannel }
        progress.onmessage(PROGRESS_EXTRACTING)
        setTimeout(() => progress.onmessage(PROGRESS_COMMITTING), 50)
        return run.promise
      },
      install_cancel: () => true,
    })

    await clickInstall(user)
    expect(await screen.findByText('Putting the mods in place…')).toBeVisible()
    expect(screen.getByRole('button', { name: 'Cancel' })).toBeDisabled()
    expect(ipc.callsOf('install_cancel')).toHaveLength(0)

    run.reject({ kind: 'installCancelled' })
    expect(await screen.findByText('Cancelled. Nothing was installed.')).toBeVisible()
  })

  it('calls install_cancel while extracting', async () => {
    const user = userEvent.setup()
    const run = deferred()
    const { ipc } = renderModal({
      install_start: (payload) => {
        ;(payload as { progress: ProgressChannel }).progress.onmessage(PROGRESS_EXTRACTING)
        return run.promise
      },
      install_cancel: () => true,
    })

    await clickInstall(user)
    await user.click(await screen.findByRole('button', { name: 'Cancel' }))
    await waitFor(() => expect(ipc.callsOf('install_cancel')).toHaveLength(1))
    run.reject({ kind: 'installCancelled' })
    expect(await screen.findByText('Cancelled. Nothing was installed.')).toBeVisible()
  })

  it('asks before installing while the game runs and retries with applyWhileRunning', async () => {
    const user = userEvent.setup()
    let attempts = 0
    const { ipc } = renderModal({
      install_start: () => {
        attempts += 1
        if (attempts === 1) throw { kind: 'gameRunning' }
        return makeOutcome(['Cool'])
      },
    })

    await clickInstall(user)
    expect(await screen.findByText('Install for next launch?')).toBeVisible()
    expect(startRequests(ipc)[0]?.applyWhileRunning).toBe(false)
    await user.click(screen.getByRole('button', { name: 'Install for next launch' }))
    expect(await screen.findByText('1 mod installed')).toBeVisible()
    expect(startRequests(ipc)[1]?.applyWhileRunning).toBe(true)
  })

  it('returns to the preview when the game prompt is declined', async () => {
    const user = userEvent.setup()
    renderModal({
      install_start: () => {
        throw { kind: 'gameRunning' }
      },
    })
    await clickInstall(user)
    await user.click(await screen.findByRole('button', { name: 'Back' }))
    expect(await screen.findByTestId('install-preview')).toBeVisible()
  })

  it('shows the folder hint for a split archive', async () => {
    renderModal({
      install_inspect: () => {
        throw { kind: 'archiveFailed', reason: 'multiVolume' }
      },
    })
    expect(await screen.findByText('This archive is split into several parts.')).toBeVisible()
    expect(
      screen.getByText('Extract it with another program and install the folder instead.'),
    ).toBeVisible()
  })

  it('inspects again once when the start finds the disk changed', async () => {
    const user = userEvent.setup()
    const { ipc } = renderModal({
      install_start: () => {
        throw { kind: 'stale' }
      },
    })
    await clickInstall(user)
    expect(await screen.findByText(/the source was read again/)).toBeVisible()
    await waitFor(() => expect(ipc.callsOf('install_inspect')).toHaveLength(2))
    expect(await screen.findByTestId('install-preview')).toBeVisible()
    expect(ipc.callsOf('install_start')).toHaveLength(1)
  })

  it('keeps the preview when another install is running', async () => {
    const user = userEvent.setup()
    renderModal({
      install_start: () => {
        throw { kind: 'installBusy' }
      },
    })
    await clickInstall(user)
    expect(await screen.findByText(/Another install is running/)).toBeVisible()
    expect(screen.getByRole('button', { name: 'Install' })).toBeEnabled()
  })

  it('offers Next when sources are waiting', async () => {
    const user = userEvent.setup()
    const { onClose } = renderModal({ install_start: () => makeOutcome(['Cool']) }, 2)
    expect(await screen.findByText('2 more waiting')).toBeVisible()
    await clickInstall(user)
    await user.click(await screen.findByRole('button', { name: 'Next' }))
    expect(onClose).toHaveBeenCalledTimes(1)
  })
})
