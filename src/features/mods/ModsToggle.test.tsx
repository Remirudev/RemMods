import { emit } from '@tauri-apps/api/event'
import { act, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { useBackendEvents } from '../../app/use-backend-events'
import type { GameStatus, ModRow, ToggleOutcome } from '../../lib/ipc/types'
import { useGameStatus } from '../../shared/game/queries'
import { makeGameStatus, makeModRow, makeSnapshot } from '../../test/fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { mockElementLayout } from '../../test/layout'
import { renderWithProviders } from '../../test/render'
import type { IpcHandler } from '../../test/types'
import { ModsPanel } from './ModsPanel'
import { useModsUiStore } from './mods-ui-store'

function Harness() {
  useBackendEvents()
  const status = useGameStatus().data
  const location = status?.location
  if (!status || location?.kind !== 'ready') return null
  return <ModsPanel status={status} install={location.install} />
}

const OFF_PAYLOAD = { livePak: 0, offPak: 1, liveCompanion: 0, offCompanion: 0 }

function offRow(row: ModRow): ModRow {
  return { ...row, state: 'disabled', payload: OFF_PAYLOAD }
}

const ALPHA = makeModRow('Alpha')
const BETA = makeModRow('Beta')
const COBALT = makeModRow('Cobalt', { toggleBlock: 'ambiguous' })
const SNAPSHOT = makeSnapshot([ALPHA, BETA, COBALT])

function outcomeOf(...rows: ModRow[]): ToggleOutcome {
  return { snapshot: makeSnapshot(rows, { revision: 2 }), skipped: [], unmanaged: 0 }
}

function reject(error: object): IpcHandler {
  return () => {
    throw error
  }
}

function renderMods(handlers: Record<string, IpcHandler>, status: GameStatus = makeGameStatus()) {
  const ipc = installIpcMock(undefined, { gameStatus: status, snapshot: SNAPSHOT, handlers })
  renderWithProviders(<Harness />, { gameStatus: status })
  return ipc
}

function deferred<T>() {
  let resolve: (value: T) => void = () => undefined
  const promise = new Promise<T>((done) => {
    resolve = done
  })
  return { promise, resolve }
}

const ALPHA_ARGS = { request: { targets: [{ modRef: 'm:alpha', key: 'alpha' }], enable: false } }

describe('toggling mods', () => {
  let restoreLayout: () => void

  beforeEach(() => {
    restoreLayout = mockElementLayout()
    useModsUiStore.setState({
      tab: 'mods',
      detailsRef: null,
      activeIndex: -1,
      selection: [],
      anchor: null,
    })
  })

  afterEach(() => {
    restoreLayout()
  })

  it('sends the ref and key, shows the new state at once and keeps the backend snapshot', async () => {
    const user = userEvent.setup()
    const pending = deferred<ToggleOutcome>()
    const ipc = renderMods({ mods_toggle: () => pending.promise })

    await user.click(await screen.findByRole('switch', { name: 'Disable Alpha' }))

    await waitFor(() => {
      expect(screen.getByRole('switch', { name: 'Enable Alpha' })).not.toBeChecked()
    })
    expect(ipc.callsOf('mods_toggle')[0]?.payload).toEqual({
      request: { ...ALPHA_ARGS.request, applyWhileRunning: false },
    })
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()

    await act(async () => {
      pending.resolve(outcomeOf(offRow(ALPHA), BETA, COBALT))
    })
    expect(screen.getByRole('switch', { name: 'Enable Alpha' })).not.toBeChecked()
    expect(screen.getByRole('switch', { name: 'Disable Beta' })).toBeChecked()
  })

  it('rolls the switch back when the backend refuses', async () => {
    const user = userEvent.setup()
    renderMods({ mods_toggle: reject({ kind: 'targetExists' }) })

    await user.click(await screen.findByRole('switch', { name: 'Disable Alpha' }))

    await waitFor(() => {
      expect(screen.getByRole('switch', { name: 'Disable Alpha' })).toBeChecked()
    })
  })

  it('asks before changing mods while the game runs, then repeats with applyWhileRunning', async () => {
    const user = userEvent.setup()
    let calls = 0
    const ipc = renderMods({
      mods_toggle: () => {
        calls += 1
        if (calls === 1) throw { kind: 'gameRunning' }
        return outcomeOf(offRow(ALPHA), BETA, COBALT)
      },
    })

    await user.click(await screen.findByRole('switch', { name: 'Disable Alpha' }))
    const dialog = await screen.findByRole('dialog')
    expect(within(dialog).getByText('TEKKEN 8 is running')).toBeVisible()
    await user.click(within(dialog).getByRole('button', { name: 'Apply for next launch' }))

    await waitFor(() => {
      expect(ipc.callsOf('mods_toggle')).toHaveLength(2)
    })
    expect(ipc.callsOf('mods_toggle')[1]?.payload).toEqual({
      request: { ...ALPHA_ARGS.request, applyWhileRunning: true },
    })
    await waitFor(() => {
      expect(screen.getByRole('switch', { name: 'Enable Alpha' })).not.toBeChecked()
    })
  })

  it('does not repeat the request when the running prompt is cancelled', async () => {
    const user = userEvent.setup()
    const ipc = renderMods({ mods_toggle: reject({ kind: 'gameRunning' }) })

    await user.click(await screen.findByRole('switch', { name: 'Disable Alpha' }))
    const dialog = await screen.findByRole('dialog')
    await user.click(within(dialog).getByRole('button', { name: 'Cancel' }))

    await waitFor(() => {
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
    })
    expect(ipc.callsOf('mods_toggle')).toHaveLength(1)
    expect(screen.getByRole('switch', { name: 'Disable Alpha' })).toBeChecked()
  })

  it('offers a retry when another RemMods holds the lock', async () => {
    const user = userEvent.setup()
    let calls = 0
    const ipc = renderMods({
      mods_toggle: () => {
        calls += 1
        if (calls === 1) throw { kind: 'busy' }
        return outcomeOf(offRow(ALPHA), BETA, COBALT)
      },
    })

    await user.click(await screen.findByRole('switch', { name: 'Disable Alpha' }))
    const dialog = await screen.findByRole('dialog')
    expect(within(dialog).getByText('Another RemMods is changing files')).toBeVisible()
    await user.click(within(dialog).getByRole('button', { name: 'Try again' }))

    await waitFor(() => {
      expect(ipc.callsOf('mods_toggle')).toHaveLength(2)
    })
    expect(ipc.callsOf('mods_toggle')[1]?.payload).toEqual({
      request: { ...ALPHA_ARGS.request, applyWhileRunning: false },
    })
  })

  it('explains a stale list in a dialog and rescans', async () => {
    const user = userEvent.setup()
    const ipc = renderMods({ mods_toggle: reject({ kind: 'stale' }) })

    await user.click(await screen.findByRole('switch', { name: 'Disable Alpha' }))

    const dialog = await screen.findByRole('dialog')
    expect(within(dialog).getByText('The mod list changed on disk')).toBeVisible()
    await waitFor(() => {
      expect(ipc.callsOf('mods_refresh').length).toBeGreaterThan(0)
    })
  })

  it('selects a range with Shift+click and disables both rows with one request', async () => {
    const user = userEvent.setup()
    const ipc = renderMods({ mods_toggle: () => outcomeOf(offRow(ALPHA), offRow(BETA), COBALT) })
    const rows = await screen.findAllByTestId('mods-row')

    await user.keyboard('{Control>}')
    await user.click(rows[0]!)
    await user.keyboard('{/Control}{Shift>}')
    await user.click(rows[1]!)
    await user.keyboard('{/Shift}')

    expect(await screen.findByText('2 selected')).toBeVisible()
    expect(rows[0]).toHaveAttribute('aria-selected', 'true')
    expect(rows[2]).toHaveAttribute('aria-selected', 'false')
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Disable' }))

    await waitFor(() => {
      expect(ipc.callsOf('mods_toggle')).toHaveLength(1)
    })
    expect(ipc.callsOf('mods_toggle')[0]?.payload).toEqual({
      request: {
        targets: [
          { modRef: 'm:alpha', key: 'alpha' },
          { modRef: 'm:beta', key: 'beta' },
        ],
        enable: false,
        applyWhileRunning: false,
      },
    })
  })

  it('clears the selection with Escape', async () => {
    const user = userEvent.setup()
    renderMods({})
    const rows = await screen.findAllByTestId('mods-row')
    await user.keyboard('{Control>}')
    await user.click(rows[0]!)
    await user.click(rows[1]!)
    await user.keyboard('{/Control}')
    expect(await screen.findByText('2 selected')).toBeVisible()

    act(() => screen.getByRole('grid').focus())
    await user.keyboard('{Escape}')

    await waitFor(() => {
      expect(screen.queryByText('2 selected')).not.toBeInTheDocument()
    })
  })

  it('toggles the active row with Space and does nothing on a blocked row', async () => {
    const user = userEvent.setup()
    const ipc = renderMods({ mods_toggle: () => outcomeOf(offRow(ALPHA), BETA, COBALT) })
    await screen.findAllByTestId('mods-row')
    act(() => screen.getByRole('grid').focus())

    await user.keyboard('{ArrowDown} ')
    await waitFor(() => {
      expect(ipc.callsOf('mods_toggle')).toHaveLength(1)
    })
    expect(ipc.callsOf('mods_toggle')[0]?.payload).toEqual({
      request: { ...ALPHA_ARGS.request, applyWhileRunning: false },
    })

    await user.keyboard('{ArrowDown}{ArrowDown} ')
    expect(ipc.callsOf('mods_toggle')).toHaveLength(1)
  })

  it('disables the switch of a blocked row and names the reason in a tooltip', async () => {
    const user = userEvent.setup()
    const ipc = renderMods({})
    await screen.findAllByTestId('mods-row')

    const blocked = screen.getByRole('switch', { name: 'Disable Cobalt' })
    expect(blocked).toBeDisabled()
    await user.hover(screen.getAllByTestId('toggle-cell')[2]!)

    expect(await screen.findByRole('tooltip')).toHaveTextContent(
      'Several mods share this name. Rename one first.',
    )
    expect(ipc.callsOf('mods_toggle')).toHaveLength(0)
  })

  it('does not open the details drawer when the switch is used', async () => {
    const user = userEvent.setup()
    renderMods({ mods_toggle: () => outcomeOf(offRow(ALPHA), BETA, COBALT) })

    await user.click(await screen.findByRole('switch', { name: 'Disable Alpha' }))

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })

  it('confirms "Disable all", sends mods_disable_all and then acknowledges the update', async () => {
    const user = userEvent.setup()
    const status = makeGameStatus({ update: { from: '100', to: '101' } })
    const ipc = renderMods(
      { mods_disable_all: () => outcomeOf(offRow(ALPHA), offRow(BETA), COBALT) },
      status,
    )

    await user.click(await screen.findByRole('button', { name: 'Disable all mods' }))
    const dialog = await screen.findByRole('dialog')
    expect(
      within(dialog).getByText('2 mods will be turned off. Unmanaged files stay loaded.'),
    ).toBeVisible()
    expect(ipc.callsOf('mods_disable_all')).toHaveLength(0)
    await user.click(within(dialog).getByRole('button', { name: 'Disable all' }))

    await waitFor(() => {
      expect(ipc.callsOf('mods_disable_all')).toHaveLength(1)
    })
    expect(ipc.callsOf('mods_disable_all')[0]?.payload).toEqual({ applyWhileRunning: false })
    await waitFor(() => {
      expect(ipc.callsOf('game_ack_update')).toHaveLength(1)
    })
  })

  it('shows the auto-refresh banner when the watcher reports its limit', async () => {
    renderMods({})
    await screen.findAllByTestId('mods-row')
    expect(screen.queryByText(/Auto-refresh is off/)).not.toBeInTheDocument()

    await act(async () => {
      await emit('watch-status-changed', { state: 'limitReached' })
    })

    expect(await screen.findByText(/Auto-refresh is off/)).toBeVisible()
  })

  it('offers Fix for a LogicMods folder whose name and files disagree', async () => {
    const user = userEvent.setup()
    const logic = makeModRow('Tool', {
      kind: 'logic',
      root: 'LogicMods',
      logicPrefixed: true,
      issues: [{ kind: 'logicDisabledButLoaded' }],
    })
    const ipc = installIpcMock(undefined, {
      snapshot: makeSnapshot([logic]),
      handlers: { mods_toggle: () => outcomeOf(offRow(logic)) },
    })
    renderWithProviders(<Harness />)

    await user.click(await screen.findByRole('tab', { name: /Issues/ }))
    await user.click(await screen.findByRole('button', { name: 'Fix' }))

    await waitFor(() => {
      expect(ipc.callsOf('mods_toggle')).toHaveLength(1)
    })
    expect(ipc.callsOf('mods_toggle')[0]?.payload).toEqual({
      request: {
        targets: [{ modRef: logic.modRef, key: logic.key }],
        enable: false,
        applyWhileRunning: false,
      },
    })
  })
})
