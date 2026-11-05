import { act, fireEvent, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { makeModRow } from '../../test/fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { mockElementLayout } from '../../test/layout'
import { renderWithProviders } from '../../test/render'
import { ColumnsMenu } from './ColumnsMenu'
import { COLUMN_LAYOUT_KEY, defaultColumnLayout } from './column-layout'
import { useColumnLayoutStore } from './column-layout-store'
import { NAME_SORTING } from './compare-mods'
import { ModTable } from './ModTable'
import { useModsUiStore } from './mods-ui-store'

const ROWS = [makeModRow('Alpha'), makeModRow('Beta')]

function renderTable() {
  installIpcMock()
  renderWithProviders(
    <>
      <ColumnsMenu />
      <ModTable
        rows={ROWS}
        onOpen={vi.fn()}
        onToggle={vi.fn()}
        onManage={vi.fn()}
        onBulk={vi.fn()}
      />
    </>,
  )
}

function header(name: string): HTMLElement {
  return screen.getByRole('columnheader', { name })
}

function headerNames(): string[] {
  return screen.getAllByRole('columnheader').map((cell) => cell.getAttribute('aria-label') ?? '')
}

function handle(column: string): HTMLElement {
  return screen.getByRole('separator', { name: `Resize the ${column} column` })
}

function saved(): { widths: Record<string, number>; hidden: string[] } | null {
  const raw = localStorage.getItem(COLUMN_LAYOUT_KEY)
  return raw === null ? null : JSON.parse(raw)
}

function drag(target: HTMLElement, from: number, to: number) {
  fireEvent.mouseDown(target, { button: 0, clientX: from })
  fireEvent.mouseMove(document, { clientX: to })
  fireEvent.mouseUp(document, { clientX: to })
}

describe('mod table columns', () => {
  let restoreLayout: () => void

  beforeEach(() => {
    localStorage.clear()
    useColumnLayoutStore.setState(defaultColumnLayout())
    useModsUiStore.setState({
      activeIndex: -1,
      selection: [],
      anchor: null,
      sorting: NAME_SORTING,
    })
    restoreLayout = mockElementLayout()
  })

  afterEach(() => {
    restoreLayout()
  })

  it('shows the default columns, with Name filling the free space', () => {
    renderTable()

    expect(headerNames()).toEqual([
      'On',
      '',
      'Name',
      'State',
      'Type',
      'Size',
      'Conflicts',
      'Same files',
      'Issues',
      '',
    ])
    expect(header('Name').style.width).toBe('')
    expect(header('State')).toHaveStyle({ width: '130px' })
    expect(screen.getByRole('grid').style.width).toBe('')
  })

  it('resizes a column with the arrow keys and saves the width', async () => {
    const user = userEvent.setup()
    renderTable()
    const state = handle('State')
    expect(state).toHaveAttribute('aria-valuenow', '130')
    expect(state).toHaveAttribute('aria-valuemin', '80')
    expect(state).toHaveAttribute('aria-orientation', 'vertical')

    state.focus()
    await user.keyboard('{ArrowRight}')
    expect(header('State')).toHaveStyle({ width: '140px' })
    expect(handle('State')).toHaveAttribute('aria-valuenow', '140')
    expect(saved()?.widths).toEqual({ state: 140 })

    await user.keyboard('{Shift>}{ArrowLeft}{/Shift}')
    expect(header('State')).toHaveStyle({ width: '90px' })
    expect(saved()?.widths).toEqual({ state: 90 })
    // The grid's own arrow keys only listen on the grid itself.
    expect(screen.getByRole('grid')).not.toHaveAttribute('aria-activedescendant')
  })

  it('resizes a column by dragging its edge and saves once the drag ends', () => {
    renderTable()

    fireEvent.mouseDown(handle('Type'), { button: 0, clientX: 300 })
    fireEvent.mouseMove(document, { clientX: 345 })
    expect(header('Type')).toHaveStyle({ width: '145px' })
    expect(document.documentElement).toHaveAttribute('data-column-resizing')
    expect(saved()).toBeNull()

    fireEvent.mouseUp(document, { clientX: 345 })
    expect(saved()?.widths).toEqual({ kind: 145 })
    expect(document.documentElement).not.toHaveAttribute('data-column-resizing')
  })

  it('stops at the minimum width', () => {
    renderTable()

    drag(handle('Size'), 500, 100)
    expect(header('Size')).toHaveStyle({ width: '70px' })
  })

  it('drags Name from the width it is drawn at, then sizes the table to fit', () => {
    renderTable()
    const drawn = { x: 64, y: 0, top: 0, left: 64, right: 464, bottom: 40, width: 400, height: 40 }
    vi.spyOn(header('Name'), 'getBoundingClientRect').mockReturnValue({
      ...drawn,
      toJSON: () => drawn,
    })

    drag(handle('Name'), 456, 476)

    expect(header('Name')).toHaveStyle({ width: '420px' })
    // On 64 + picture 60 + Name 420 + State 130 + Type 100 + Size 100 + Conflicts 120
    // + Same files 140 + Issues 110 + menu 52.
    expect(screen.getByRole('grid')).toHaveStyle({ width: '1296px' })
    expect(saved()?.widths).toEqual({ name: 420 })
  })

  it('does not fix the width of Name on a click without a move', () => {
    renderTable()

    drag(handle('Name'), 456, 456)

    expect(header('Name').style.width).toBe('')
    expect(saved()).toBeNull()
  })

  it('goes back to the default width on a double click', async () => {
    const user = userEvent.setup()
    renderTable()
    handle('State').focus()
    await user.keyboard('{ArrowRight}')
    drag(handle('Name'), 100, 160)

    fireEvent.doubleClick(handle('State'))
    fireEvent.doubleClick(handle('Name'))

    expect(header('State')).toHaveStyle({ width: '130px' })
    expect(header('Name').style.width).toBe('')
    expect(saved()?.widths).toEqual({})
  })

  it('hides and shows columns from the Columns menu and resets them', async () => {
    const user = userEvent.setup()
    renderTable()

    await user.click(screen.getByRole('button', { name: 'Columns' }))
    const menu = screen.getByRole('dialog', { name: 'Columns' })
    expect(within(menu).getByRole('checkbox', { name: 'On' })).toBeDisabled()
    expect(within(menu).getByRole('checkbox', { name: 'Name' })).toBeDisabled()
    expect(within(menu).getByRole('checkbox', { name: 'Version' })).not.toBeChecked()

    await user.click(within(menu).getByRole('checkbox', { name: 'Type' }))
    await user.click(within(menu).getByRole('checkbox', { name: 'Version' }))

    expect(screen.queryByRole('columnheader', { name: 'Type' })).not.toBeInTheDocument()
    expect(header('Version')).toBeVisible()
    expect(saved()?.hidden).toEqual(['kind', 'category'])
    const firstRow = screen.getAllByTestId('mods-row')[0]!
    expect(firstRow.querySelectorAll('td')).toHaveLength(10)

    await user.click(within(menu).getByRole('button', { name: 'Reset columns' }))

    expect(header('Type')).toBeVisible()
    expect(screen.queryByRole('columnheader', { name: 'Version' })).not.toBeInTheDocument()
    expect(saved()).toBeNull()
  })

  it('reaches the Columns menu with the keyboard', async () => {
    const user = userEvent.setup()
    renderTable()

    screen.getByRole('button', { name: 'Columns' }).focus()
    await user.keyboard('{Enter}')
    const menu = await screen.findByRole('dialog', { name: 'Columns' })
    const kind = within(menu).getByRole('checkbox', { name: 'Type' })
    kind.focus()
    await user.keyboard(' ')

    expect(kind).not.toBeChecked()
    expect(screen.queryByRole('columnheader', { name: 'Type' })).not.toBeInTheDocument()
  })

  it('sorts by Name again when the sorted column is hidden', async () => {
    const user = userEvent.setup()
    renderTable()
    await user.click(screen.getByRole('button', { name: /^Type/ }))
    expect(header('Type')).toHaveAttribute('aria-sort', 'ascending')

    act(() => useColumnLayoutStore.getState().setColumnVisible('kind', false))

    expect(header('Name')).toHaveAttribute('aria-sort', 'ascending')
  })
})
