import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { makeBulkRow, makeModRow } from '../../test/fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { renderWithProviders } from '../../test/render'
import { BulkResultList } from './BulkResultList'
import { BulkResultsBody } from './BulkResultsBody'
import { rowSubject } from './manage-model'

const ROWS = [
  makeBulkRow('Alpha', { kind: 'trashed', items: 1, trash: 'home' }),
  makeBulkRow('Bravo', { kind: 'trashed', items: 1, trash: 'recycleBin' }),
  makeBulkRow('Charlie', { kind: 'deleted', leftover: null }),
  makeBulkRow('Delta', { kind: 'deleted', leftover: 'Mods/_delete_x' }),
  makeBulkRow('Echo', { kind: 'unavailable', reason: 'recycleRefused', size: 2048 }),
  makeBulkRow('Foxtrot', { kind: 'failed', error: { kind: 'busyCancelled' } }),
  makeBulkRow('Golf', { kind: 'notRun' }),
]

function rowText(name: string): string {
  const row = screen
    .getAllByTestId('bulk-result-row')
    .find((candidate) => within(candidate).queryByText(name))
  return row?.textContent ?? ''
}

describe('BulkResultList', () => {
  it('renders every result kind with its text', () => {
    installIpcMock()
    renderWithProviders(<BulkResultList rows={ROWS} />)

    expect(rowText('Alpha')).toContain('Moved to the Trash')
    expect(rowText('Bravo')).toContain('Moved to the Recycle Bin')
    expect(rowText('Charlie')).toContain('Deleted')
    expect(rowText('Delta')).toContain('a hidden part is left: Mods/_delete_x')
    expect(rowText('Echo')).toContain('Windows would not move this folder to the Recycle Bin')
    expect(rowText('Echo')).toContain('2 KB')
    expect(rowText('Foxtrot')).toContain(
      'Cancelled while waiting for the folder; this mod was not changed.',
    )
    expect(rowText('Golf')).toContain('Not attempted (cancelled)')
  })

  it('says an unavailable loose row cannot be deleted here', () => {
    installIpcMock()
    const loose = rowSubject(makeModRow('Loose', { kind: 'loose' }))
    renderWithProviders(
      <BulkResultList
        rows={[makeBulkRow('Loose', { kind: 'unavailable', reason: 'otherDevice', size: 10 })]}
        subjects={[loose]}
      />,
    )
    expect(rowText('Loose')).toContain('cannot be deleted here: Loose files have no folder')
  })
})

describe('BulkResultsBody', () => {
  it('offers only the deletable unavailable rows with reason, size and total', async () => {
    const user = userEvent.setup()
    installIpcMock()
    const onDelete = vi.fn()
    const subjects = [
      rowSubject(makeModRow('Alpha')),
      rowSubject(makeModRow('Bravo')),
      rowSubject(makeModRow('Loose', { kind: 'loose' })),
    ]
    renderWithProviders(
      <BulkResultsBody
        results={{
          action: 'trash',
          subjects,
          rows: [
            makeBulkRow('Alpha', { kind: 'trashed', items: 1, trash: 'home' }),
            makeBulkRow('Bravo', { kind: 'unavailable', reason: 'otherDevice', size: 4096 }),
            makeBulkRow('Loose', { kind: 'unavailable', reason: 'otherDevice', size: 10 }),
          ],
        }}
        onClose={vi.fn()}
        onDelete={onDelete}
      />,
    )

    const warning = screen.getByTestId('bulk-delete-warning')
    expect(within(warning).getByText('Bravo')).toBeInTheDocument()
    expect(within(warning).queryByText('Loose')).toBeNull()
    expect(within(warning).getByText(/on another drive than your Trash/)).toBeInTheDocument()
    expect(within(warning).getByText('This mod (4 KB) would be deleted permanently instead.'))
    expect(within(warning).getByText('This cannot be undone.')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Delete permanently (1)' }))
    expect(onDelete).toHaveBeenCalledOnce()
  })

  it('has only Close after a delete', () => {
    installIpcMock()
    renderWithProviders(
      <BulkResultsBody
        results={{ action: 'delete', rows: [makeBulkRow('Alpha', { kind: 'notRun' })] }}
        onClose={vi.fn()}
        onDelete={vi.fn()}
      />,
    )
    expect(screen.getByRole('button', { name: 'Close' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /Delete permanently/ })).toBeNull()
  })
})
