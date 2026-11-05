import { screen, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import type { SkinCopy } from '../../lib/ipc/types'
import { installIpcMock } from '../../test/ipc-mock'
import { renderWithProviders } from '../../test/render'
import { SkinCopiesDialog } from './SkinCopiesDialog'

const TAKEN = '2026-11-05T12:00:00.000Z'

function renderCopies(copies: () => SkinCopy[]) {
  installIpcMock(undefined, { handlers: { skins_copies: copies } })
  renderWithProviders(<SkinCopiesDialog />)
}

describe('SkinCopiesDialog', () => {
  it('names where each copy came from and when', async () => {
    renderCopies(() => [
      { id: 'a-1', taken: TAKEN, setName: 'Fine mods' },
      { id: 'game-1', taken: TAKEN, setName: null },
    ])

    const [earlier, before] = await screen.findAllByTestId('skin-copy')
    if (!earlier || !before) throw new Error('two rows')
    expect(within(earlier).getByText('Earlier copy of Fine mods')).toBeVisible()
    expect(within(before).getByText('Skins from before a switch')).toBeVisible()
    expect(within(earlier).getByText(new Date(TAKEN).toLocaleString('en'))).toBeVisible()
    expect(within(before).getByRole('button', { name: 'Keep as a set' })).toBeVisible()
  })

  it('says when there are none yet', async () => {
    renderCopies(() => [])
    expect(await screen.findByText(/first one is made at your first switch/)).toBeVisible()
  })

  it('says when the list cannot be read', async () => {
    renderCopies(() => {
      throw { kind: 'internal' }
    })
    expect(await screen.findByRole('alert')).toHaveTextContent('The copies could not be read.')
  })
})
