import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { GameStatus } from '../../lib/ipc/types'
import { useGameStatus } from '../../shared/game/queries'
import { makeGameStatus, makeModRow, makeSnapshot } from '../../test/fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { mockElementLayout } from '../../test/layout'
import { renderWithProviders } from '../../test/render'
import { ModsPanel } from './ModsPanel'
import { ModTable } from './ModTable'
import { useModsUiStore } from './mods-ui-store'

function Harness() {
  const status = useGameStatus().data
  const location = status?.location
  if (!status || location?.kind !== 'ready') return null
  return <ModsPanel status={status} install={location.install} />
}

const ALPHA = makeModRow('Alpha', { sizeBytes: 1024 })
const LOOSE = makeModRow('Gamma', { kind: 'loose' })
const LINKED = makeModRow('Linked', { toggleBlock: 'linked', issues: [{ kind: 'linkedMod' }] })
const SNAPSHOT = makeSnapshot([ALPHA, LOOSE, LINKED])

function renderMods(status: GameStatus = makeGameStatus()) {
  const ipc = installIpcMock(undefined, { gameStatus: status, snapshot: SNAPSHOT })
  renderWithProviders(<Harness />, { gameStatus: status })
  return ipc
}

async function ctrlClick(user: ReturnType<typeof userEvent.setup>, name: string) {
  const row = (await screen.findAllByTestId('mods-row')).find((candidate) =>
    within(candidate).queryByText(name),
  )
  if (!row) throw new Error(`no row ${name}`)
  await user.keyboard('{Control>}')
  await user.click(row)
  await user.keyboard('{/Control}')
}

describe('bulk actions in the ActionBar', () => {
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

  afterEach(() => restoreLayout())

  it('shows trash and delete with the counts the matrix allows', async () => {
    const user = userEvent.setup()
    renderMods()
    await ctrlClick(user, 'Alpha')
    await ctrlClick(user, 'Gamma')
    await ctrlClick(user, 'Linked')

    const bar = await screen.findByRole('toolbar', { name: 'Selected mods' })
    expect(within(bar).getByRole('button', { name: 'Move to Trash (2)' })).toBeEnabled()
    expect(within(bar).getByRole('button', { name: 'Delete permanently… (1)' })).toBeEnabled()
  })

  it('opens the first delete confirm with the skipped line and calls nothing', async () => {
    const user = userEvent.setup()
    const ipc = renderMods()
    await ctrlClick(user, 'Alpha')
    await ctrlClick(user, 'Gamma')

    const bar = await screen.findByRole('toolbar', { name: 'Selected mods' })
    await user.click(within(bar).getByRole('button', { name: 'Delete permanently… (1)' }))

    const dialog = await screen.findByRole('dialog')
    expect(within(dialog).getByText(/1 selected mod is skipped/)).toBeVisible()
    expect(within(dialog).getByText('Alpha')).toBeVisible()
    await user.click(within(dialog).getByRole('button', { name: 'Cancel' }))
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull())
    expect(ipc.callsOf('mods_delete_prepare_many')).toHaveLength(0)
    expect(ipc.callsOf('mods_delete_many')).toHaveLength(0)
  })
})

describe('bulk keys in the table', () => {
  let restoreLayout: () => void

  beforeEach(() => {
    restoreLayout = mockElementLayout()
  })

  afterEach(() => restoreLayout())

  it('Shift+Delete on a loose active row among selected rows calls onBulk', async () => {
    const user = userEvent.setup()
    const onBulk = vi.fn()
    const onManage = vi.fn()
    installIpcMock()
    useModsUiStore.setState({
      activeIndex: 1,
      selection: ['m:alpha', 'm:gamma'],
      anchor: 'm:alpha',
    })
    renderWithProviders(
      <ModTable
        rows={[ALPHA, LOOSE]}
        onOpen={vi.fn()}
        onToggle={vi.fn()}
        onManage={onManage}
        onBulk={onBulk}
      />,
    )

    screen.getByRole('grid', { name: 'Installed mods' }).focus()
    await user.keyboard('{Shift>}{Delete}{/Shift}')
    expect(onBulk).toHaveBeenCalledWith('delete')
    await user.keyboard('{Delete}')
    expect(onBulk).toHaveBeenLastCalledWith('trash')
    expect(onManage).not.toHaveBeenCalled()
  })

  it('Shift+Delete on a lone loose row does nothing', async () => {
    const user = userEvent.setup()
    const onBulk = vi.fn()
    const onManage = vi.fn()
    installIpcMock()
    useModsUiStore.setState({ activeIndex: 1, selection: ['m:gamma'], anchor: 'm:gamma' })
    renderWithProviders(
      <ModTable
        rows={[ALPHA, LOOSE]}
        onOpen={vi.fn()}
        onToggle={vi.fn()}
        onManage={onManage}
        onBulk={onBulk}
      />,
    )

    screen.getByRole('grid', { name: 'Installed mods' }).focus()
    await user.keyboard('{Shift>}{Delete}{/Shift}')
    expect(onBulk).not.toHaveBeenCalled()
    expect(onManage).not.toHaveBeenCalled()
  })
})
