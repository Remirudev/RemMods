import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import type { GameStatus, PlatformInfo, RepairList } from '../../lib/ipc/types'
import { useGameStatus } from '../../shared/game/queries'
import {
  makeGameStatus,
  makeModRow,
  makeSnapshot,
  WINDOWS_PLATFORM_INFO,
} from '../../test/fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { mockElementLayout } from '../../test/layout'
import { renderWithProviders } from '../../test/render'
import type { IpcHandler } from '../../test/types'
import { ModsPanel } from './ModsPanel'
import { useModsUiStore } from './mods-ui-store'

function Harness() {
  const status = useGameStatus().data
  const location = status?.location
  if (!status || location?.kind !== 'ready') return null
  return <ModsPanel status={status} install={location.install} />
}

const ALPHA = makeModRow('Alpha')
const LINKED = makeModRow('Linked', { toggleBlock: 'linked', issues: [{ kind: 'linkedMod' }] })
const SNAPSHOT = makeSnapshot([ALPHA, LINKED])
const TRASHED = {
  snapshot: makeSnapshot([LINKED], { revision: 2 }),
  key: 'alpha',
  items: 1,
  kind: 'home',
}

function renderMods(
  handlers: Record<string, IpcHandler>,
  status: GameStatus = makeGameStatus(),
  platformInfo?: PlatformInfo,
) {
  const ipc = installIpcMock(undefined, { gameStatus: status, snapshot: SNAPSHOT, handlers })
  renderWithProviders(<Harness />, { gameStatus: status, platformInfo })
  return ipc
}

async function openMenuOf(user: ReturnType<typeof userEvent.setup>, name: string) {
  await user.click(await screen.findByRole('button', { name: `Actions for ${name}` }))
}

describe('mod management in the table', () => {
  let restoreLayout: () => void

  beforeEach(() => {
    restoreLayout = mockElementLayout()
    useModsUiStore.setState({ tab: 'mods', detailsRef: null, activeIndex: -1, selection: [] })
  })

  afterEach(() => restoreLayout())

  it('moves a mod to the Trash after one confirm and calls nothing else', async () => {
    const user = userEvent.setup()
    const ipc = renderMods({ mods_trash: () => TRASHED })

    await openMenuOf(user, 'Alpha')
    await user.click(await screen.findByRole('menuitem', { name: /Move to Trash/ }))
    const dialog = await screen.findByRole('dialog')
    expect(within(dialog).getByText(/to the Trash\? You can restore/)).toBeVisible()
    await user.click(within(dialog).getByRole('button', { name: 'Move to Trash' }))

    await waitFor(() => expect(ipc.callsOf('mods_trash')).toHaveLength(1))
    expect(ipc.callsOf('mods_trash')[0]?.payload).toEqual({
      request: { target: { modRef: 'm:alpha', key: 'alpha' }, applyWhileRunning: false },
    })
    expect(ipc.callsOf('mods_delete_prepare')).toHaveLength(0)
    expect(ipc.callsOf('mods_delete_permanently')).toHaveLength(0)
  })

  it('names the Recycle Bin on Windows and the Trash elsewhere', async () => {
    const user = userEvent.setup()
    renderMods({}, makeGameStatus(), WINDOWS_PLATFORM_INFO)

    await openMenuOf(user, 'Alpha')
    await user.click(await screen.findByRole('menuitem', { name: /Move to Recycle Bin/ }))
    const dialog = await screen.findByRole('dialog', { name: 'Move to Recycle Bin?' })
    expect(within(dialog).getByText(/to the Recycle Bin\? You can restore/)).toBeVisible()
    expect(within(dialog).getByRole('button', { name: 'Move to Recycle Bin' })).toBeVisible()
  })

  it('disables every action of a linked mod', async () => {
    const user = userEvent.setup()
    renderMods({})

    await openMenuOf(user, 'Linked')

    for (const name of [/Rename/, /Move to Trash/, /Delete permanently/]) {
      expect(await screen.findByRole('menuitem', { name })).toBeDisabled()
    }
  })

  it('opens the rename dialog on F2 for the active row', async () => {
    const user = userEvent.setup()
    renderMods({})
    await screen.findAllByTestId('mods-row')

    await user.click(screen.getByRole('grid', { name: 'Installed mods' }))
    await user.keyboard('{ArrowDown}{F2}')

    expect(await screen.findByRole('textbox', { name: 'New name' })).toHaveValue('Alpha')
  })

  it('shows the Repair tab with its count only when there is something to repair', async () => {
    const list: RepairList = {
      revision: 1,
      items: [
        {
          id: 'leftover|l:1',
          kind: 'leftover',
          target: { modRef: 'l:1', key: 'Mods/_x_tmp_1' },
          label: 'Mods/_x_tmp_1',
          actions: ['trash', 'deletePermanently', 'reveal'],
          suggestedName: null,
        },
      ],
    }
    const user = userEvent.setup()
    renderMods({ repair_list: () => list })

    const tab = await screen.findByRole('tab', { name: /Repair\s*1/ })
    await user.click(tab)
    expect(await screen.findByText('Leftover folder')).toBeVisible()
    expect(screen.getByRole('button', { name: 'Delete permanently…' })).toBeVisible()
  })

  it('hides the Repair tab while nothing needs repair', async () => {
    renderMods({ repair_list: () => ({ revision: 1, items: [] }) })

    await screen.findAllByTestId('mods-row')
    expect(screen.queryByRole('tab', { name: /Repair/ })).toBeNull()
  })
})
