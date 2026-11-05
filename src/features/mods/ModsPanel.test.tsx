import { emit } from '@tauri-apps/api/event'
import { act, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { useBackendEvents } from '../../app/use-backend-events'
import type { GameStatus } from '../../lib/ipc/types'
import { useGameStatus } from '../../shared/game/queries'
import {
  INSTALL_WITHOUT_MANIFEST,
  makeGameStatus,
  makeModRow,
  makeSnapshot,
  TEST_INSTALL,
} from '../../test/fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { mockElementLayout } from '../../test/layout'
import { renderWithProviders } from '../../test/render'
import type { IpcHandler, IpcMockOptions } from '../../test/types'
import { NAME_SORTING } from './compare-mods'
import { ModsPanel } from './ModsPanel'
import { useModsUiStore } from './mods-ui-store'

// Like ModsGate: the panel reads the cached game status, so events that update it show up.
function Harness() {
  useBackendEvents()
  const status = useGameStatus().data
  const location = status?.location
  if (!status || location?.kind !== 'ready') return null
  return <ModsPanel status={status} install={location.install} />
}

function renderMods(status: GameStatus = makeGameStatus(), options: IpcMockOptions = {}) {
  const ipc = installIpcMock(undefined, { gameStatus: status, ...options })
  const rendered = renderWithProviders(<Harness />, { gameStatus: status })
  return { ipc, ...rendered }
}

function reject(error: object): IpcHandler {
  return () => {
    throw error
  }
}

function rowNames(): string[] {
  return screen.getAllByTestId('mods-row').map((row) => row.querySelector('bdi')?.textContent ?? '')
}

function rowNamed(name: string): HTMLElement {
  const row = screen
    .getAllByTestId('mods-row')
    .find((candidate) => candidate.querySelector('bdi')?.textContent === name)
  if (!row) throw new Error(`no row ${name}`)
  return row
}

describe('ModsPanel', () => {
  let restoreLayout: () => void

  beforeEach(() => {
    restoreLayout = mockElementLayout()
    useModsUiStore.setState({
      filter: {
        text: '',
        state: 'all',
        kinds: [],
        characters: [],
        conflictsOnly: false,
        contentConflictsOnly: false,
        issuesOnly: false,
      },
      sorting: NAME_SORTING,
      tab: 'mods',
      detailsRef: null,
      activeIndex: -1,
    })
  })

  afterEach(() => {
    restoreLayout()
  })

  it('renders the rows sorted by name with tab counts', async () => {
    renderMods()

    await screen.findAllByTestId('mods-row')
    expect(rowNames()).toEqual(['Alpha Skin', 'Beta Stage', 'Gamma Logic'])
    expect(screen.getByRole('tab', { name: /Mods\s*3/ })).toBeVisible()
    expect(screen.getByRole('tab', { name: /Issues\s*1/ })).toBeVisible()
    expect(screen.getByRole('grid', { name: 'Installed mods' })).toHaveAttribute(
      'aria-rowcount',
      '4',
    )
  })

  it('sorts by a column header and flips the direction', async () => {
    const user = userEvent.setup()
    renderMods()
    await screen.findAllByTestId('mods-row')

    await user.click(screen.getByRole('button', { name: /^Name/ }))

    expect(rowNames()).toEqual(['Gamma Logic', 'Beta Stage', 'Alpha Skin'])
    expect(screen.getByRole('columnheader', { name: /Name/ })).toHaveAttribute(
      'aria-sort',
      'descending',
    )
  })

  it('reduces the rows and the count text with the search filter', async () => {
    const user = userEvent.setup()
    renderMods()
    await screen.findAllByTestId('mods-row')
    expect(screen.getByTestId('mods-count')).toHaveTextContent('3 of 3 mods')

    await user.type(screen.getByRole('textbox', { name: 'Search mods' }), 'beta')

    expect(rowNames()).toEqual(['Beta Stage'])
    expect(screen.getByTestId('mods-count')).toHaveTextContent('1 of 3 mod')
  })

  it('offers to clear the filters when nothing matches', async () => {
    const user = userEvent.setup()
    renderMods()
    await screen.findAllByTestId('mods-row')

    await user.type(screen.getByRole('textbox', { name: 'Search mods' }), 'nothing like this')

    expect(screen.getByText('No mod matches the filters.')).toBeVisible()
    await user.click(screen.getAllByRole('button', { name: 'Clear filters' })[0]!)
    expect(await screen.findAllByTestId('mods-row')).toHaveLength(3)
  })

  it('toggles the conflicts-only filter from the header chip', async () => {
    const user = userEvent.setup()
    renderMods()
    await screen.findAllByTestId('mods-row')

    const chip = screen.getByRole('button', { name: '1 mod in conflict' })
    expect(chip).toHaveAttribute('aria-pressed', 'false')

    await user.click(chip)
    expect(rowNames()).toEqual(['Beta Stage'])
    expect(chip).toHaveAttribute('aria-pressed', 'true')

    await user.click(chip)
    expect(rowNames()).toHaveLength(3)
  })

  it('counts only shared upper-body or full-body clothing in the same-files column and filter', async () => {
    const user = userEvent.setup()
    const link = { modRef: 'm:other', key: 'other', name: 'Other', shared: 2, sample: [] }
    const same = makeModRow('Same', { contentConflicts: [{ ...link, outfit: 1 }] })
    const shoes = makeModRow('Shoes', { contentConflicts: [{ ...link, outfit: 0 }] })
    renderMods(makeGameStatus(), {
      snapshot: makeSnapshot([same, shoes, makeModRow('Plain')]),
    })
    await screen.findAllByTestId('mods-row')

    expect(screen.getByRole('columnheader', { name: 'Same files' })).toBeVisible()
    expect(screen.getAllByText('1 mod')).toHaveLength(1)
    expect(within(rowNamed('Same')).getByText('1 mod')).toBeVisible()

    const filter = screen.getByRole('checkbox', { name: 'Replaces same files' })
    expect(filter).toHaveAccessibleDescription(/only upper-body and full-body clothing/)
    await user.click(filter)
    expect(rowNames()).toEqual(['Same'])
  })

  it('opens the details drawer with ArrowDown and Enter, ignores Space, closes with Escape', async () => {
    const user = userEvent.setup()
    const { ipc } = renderMods()
    await screen.findAllByTestId('mods-row')
    const grid = screen.getByRole('grid')
    act(() => grid.focus())

    await user.keyboard(' ')
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
    expect(ipc.callsOf('mods_details')).toHaveLength(0)

    await user.keyboard('{ArrowDown}')
    expect(grid).toHaveAttribute('aria-activedescendant', 'mods-row-0')
    await user.keyboard('{Enter}')

    const drawer = await screen.findByRole('dialog')
    expect(within(drawer).getByText('Alpha Skin')).toBeVisible()
    expect(await within(drawer).findByText('alpha.pak')).toBeVisible()
    expect(ipc.callsOf('mods_details')[0]?.payload).toEqual({ modRef: 'm:alpha skin' })

    await user.keyboard('{Escape}')
    await waitFor(() => {
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
    })
  })

  it('opens the drawer by clicking a row and reveals the mod', async () => {
    const user = userEvent.setup()
    const { ipc } = renderMods()
    const rows = await screen.findAllByTestId('mods-row')

    await user.click(rows[1]!)
    const drawer = await screen.findByRole('dialog')
    expect(within(drawer).getByText('Beta Stage')).toBeVisible()
    await user.click(within(drawer).getByRole('button', { name: 'Show in file manager' }))

    await waitFor(() => {
      expect(ipc.callsOf('mods_reveal')[0]?.payload).toEqual({ modRef: 'm:beta stage' })
    })
  })

  it('re-detects the game once when the scan reports a missing folder', async () => {
    const { ipc } = renderMods(undefined, {
      handlers: { mods_list: reject({ kind: 'io', op: 'scan', class: 'notFound' }) },
    })

    expect(await screen.findByText('The mod folders could not be read')).toBeVisible()
    expect(screen.getByText('The file or folder was not found.')).toBeVisible()
    await waitFor(() => {
      expect(ipc.callsOf('game_detect')).toHaveLength(1)
    })
  })

  it('does not re-detect for other scan errors', async () => {
    const { ipc } = renderMods(undefined, {
      handlers: { mods_list: reject({ kind: 'io', op: 'scan', class: 'accessDenied' }) },
    })

    expect(await screen.findByText('Access was denied.')).toBeVisible()
    expect(ipc.callsOf('game_detect')).toHaveLength(0)
  })

  it('acknowledges an update notice with game_ack_update', async () => {
    const user = userEvent.setup()
    const status = makeGameStatus({ update: { from: '100', to: '200' } })
    const { ipc } = renderMods(status)

    expect(await screen.findByText(/changed from build 100 to build 200/)).toBeVisible()
    await user.click(screen.getByRole('button', { name: 'Got it' }))

    await waitFor(() => {
      expect(ipc.callsOf('game_ack_update')).toHaveLength(1)
    })
  })

  it('warns when another mod manager is running', async () => {
    renderMods(makeGameStatus({ otherManager: true }))

    expect(await screen.findByText('Another mod manager is running')).toBeVisible()
  })

  it('launches the game and shows the starting state', async () => {
    const user = userEvent.setup()
    const { ipc } = renderMods()

    await user.click(await screen.findByRole('button', { name: 'Launch TEKKEN 8' }))

    await waitFor(() => {
      expect(ipc.callsOf('game_launch')).toHaveLength(1)
    })
    expect(await screen.findByText('Starting…')).toBeInTheDocument()
  })

  it('disables the launch button while the game is running', async () => {
    renderMods(makeGameStatus({ running: 'running' }))

    expect(await screen.findByRole('button', { name: 'Running' })).toBeDisabled()
    expect(screen.getByTestId('running-badge')).toHaveTextContent('Running')
  })

  it('explains that Steam starts another installation when there is no manifest', async () => {
    const status = makeGameStatus({
      location: { kind: 'ready', install: { ...TEST_INSTALL, ...INSTALL_WITHOUT_MANIFEST } },
    })
    renderMods(status)

    expect(
      await screen.findByText('Steam starts its own TEKKEN 8 installation, not this folder.'),
    ).toBeVisible()
  })

  it('refetches the list when mods-changed reports a new revision', async () => {
    const { ipc } = renderMods()
    await screen.findAllByTestId('mods-row')
    await waitFor(() => {
      expect(ipc.callsOf('mods_list').length).toBeGreaterThan(0)
    })
    const before = ipc.callsOf('mods_list').length

    await act(async () => {
      await emit('mods-changed', { revision: 2 })
    })

    await waitFor(() => {
      expect(ipc.callsOf('mods_list').length).toBeGreaterThan(before)
    })
  })

  it('leaves the list alone when mods-changed reports the revision it already has', async () => {
    const { ipc } = renderMods()
    await screen.findAllByTestId('mods-row')
    await waitFor(() => {
      expect(ipc.callsOf('mods_list').length).toBeGreaterThan(0)
    })
    const before = ipc.callsOf('mods_list').length

    await act(async () => {
      await emit('mods-changed', { revision: 1 })
    })

    expect(ipc.callsOf('mods_list')).toHaveLength(before)
  })

  it('updates the running badge on game-status-changed', async () => {
    renderMods()
    await screen.findAllByTestId('mods-row')
    expect(screen.getByTestId('running-badge')).toHaveTextContent('Not running')

    await act(async () => {
      await emit('game-status-changed', {
        running: 'running',
        session: 1,
        otherManager: false,
        update: null,
      })
    })

    await waitFor(() => {
      expect(screen.getByTestId('running-badge')).toHaveTextContent('Running')
    })
  })
})
