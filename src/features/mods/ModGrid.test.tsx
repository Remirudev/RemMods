import { act, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { ModRow } from '../../lib/ipc/types'
import { makeModItem, makeModRow } from '../../test/fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { mockElementLayout } from '../../test/layout'
import { renderWithProviders } from '../../test/render'
import { NAME_SORTING } from './compare-mods'
import { ModGrid } from './ModGrid'
import { useModsUiStore } from './mods-ui-store'

const ROWS: ModRow[] = [
  makeModRow('Bravo'),
  makeModRow('Alpha', { state: 'disabled' }),
  makeModRow('Charlie'),
]

function renderGrid(rows: ModRow[] = ROWS) {
  const handlers = {
    onOpen: vi.fn(),
    onToggle: vi.fn(),
    onManage: vi.fn(),
    onBulk: vi.fn(),
  }
  const ipc = installIpcMock(undefined, {
    handlers: {
      mods_items: (payload) => ({
        modRef: (payload as { modRef: string }).modRef,
        ownPicture: null,
        items: [makeModItem('ker_bdf_a', { name: 'Long coat' })],
      }),
    },
  })
  renderWithProviders(<ModGrid rows={rows} {...handlers} />)
  return { ...handlers, ipc }
}

function cardNames() {
  return screen.getAllByTestId('mods-card').map((card) => card.querySelector('bdi')?.textContent)
}

describe('ModGrid', () => {
  let restoreLayout: () => void

  beforeEach(() => {
    restoreLayout = mockElementLayout()
    useModsUiStore.setState({
      sorting: NAME_SORTING,
      activeIndex: -1,
      selection: [],
      anchor: null,
    })
  })

  afterEach(() => {
    restoreLayout()
  })

  it('shows the cards in the list’s order', async () => {
    renderGrid()
    await screen.findAllByTestId('mods-card')
    expect(cardNames()).toEqual(['Alpha', 'Bravo', 'Charlie'])
  })

  it('follows the sort the table uses', async () => {
    useModsUiStore.setState({ sorting: [{ id: 'name', desc: true }] })
    renderGrid()
    await screen.findAllByTestId('mods-card')
    expect(cardNames()).toEqual(['Charlie', 'Bravo', 'Alpha'])
  })

  it('says what a mod replaces and for whom', async () => {
    renderGrid()
    const [card] = await screen.findAllByTestId('mods-card')
    if (!card) throw new Error('no card')
    expect(await within(card).findByText('Long coat')).toBeVisible()
  })

  it('opens a mod on click, and selects with Ctrl and Shift without opening', async () => {
    const user = userEvent.setup()
    const { onOpen } = renderGrid()
    const cards = await screen.findAllByTestId('mods-card')
    const [first, , third] = cards
    if (!first || !third) throw new Error('no cards')

    await user.click(first)
    expect(onOpen).toHaveBeenCalledWith('m:alpha')
    onOpen.mockClear()

    await user.keyboard('{Control>}')
    await user.click(third)
    await user.keyboard('{/Control}')
    expect(onOpen).not.toHaveBeenCalled()
    expect(useModsUiStore.getState().selection).toEqual(['m:alpha', 'm:charlie'])
    expect(third).toHaveAttribute('aria-selected', 'true')
  })

  it('never treats the switch or the menu as a click on the card', async () => {
    const user = userEvent.setup()
    const { onOpen, onToggle } = renderGrid()
    await screen.findAllByTestId('mods-card')

    await user.click(screen.getByRole('switch', { name: 'Disable Bravo' }))
    await user.click(screen.getByRole('button', { name: 'Actions for Bravo' }))

    expect(onToggle).toHaveBeenCalledTimes(1)
    expect(onOpen).not.toHaveBeenCalled()
  })

  it('works from the keyboard like the table', async () => {
    const user = userEvent.setup()
    const { onOpen, onToggle } = renderGrid()
    await screen.findAllByTestId('mods-card')
    const grid = screen.getByRole('grid')
    act(() => grid.focus())

    await user.keyboard('{ArrowDown}')
    expect(grid).toHaveAttribute('aria-activedescendant', 'mods-card-0')
    await user.keyboard('{ArrowDown}')
    expect(grid).toHaveAttribute('aria-activedescendant', 'mods-card-1')

    await user.keyboard(' ')
    expect(onToggle).toHaveBeenCalledTimes(1)

    await user.keyboard('{Enter}')
    expect(onOpen).toHaveBeenCalledWith('m:bravo')

    await user.keyboard('{Shift>}{ArrowDown}{/Shift}')
    expect(useModsUiStore.getState().selection).toEqual(['m:bravo', 'm:charlie'])
    await user.keyboard('{Escape}')
    expect(useModsUiStore.getState().selection).toEqual([])
  })

  it('leaves on and off to the switch and names only the other states', async () => {
    renderGrid([
      makeModRow('On'),
      makeModRow('Off', { state: 'disabled' }),
      makeModRow('Half', { state: 'partial' }),
    ])
    const [half, off, on] = await screen.findAllByTestId('mods-card')
    if (!half || !off || !on) throw new Error('no cards')

    expect(within(on).queryByText('Enabled')).not.toBeInTheDocument()
    expect(within(off).queryByText('Disabled')).not.toBeInTheDocument()
    expect(within(half).getByText('Partial')).toBeVisible()
  })

  it('opens the row menu at the pointer on a right-click and leaves the selection alone', async () => {
    const user = userEvent.setup()
    const { onOpen, onManage } = renderGrid()
    const [card] = await screen.findAllByTestId('mods-card')
    if (!card) throw new Error('no card')

    await user.pointer({ keys: '[MouseRight]', target: card })
    await user.click(await screen.findByRole('menuitem', { name: /Rename/ }))

    expect(onManage).toHaveBeenCalledWith('rename', expect.objectContaining({ name: 'Alpha' }))
    expect(onOpen).not.toHaveBeenCalled()
    expect(useModsUiStore.getState().selection).toEqual([])
  })

  it('marks the active and the switched-off cards', async () => {
    useModsUiStore.setState({ activeIndex: 1 })
    renderGrid()
    const [off, active] = await screen.findAllByTestId('mods-card')
    expect(off).toHaveAttribute('data-off')
    expect(active).toHaveAttribute('data-active')
  })
})
