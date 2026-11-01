import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import { makeCandidate, makeGameStatus } from '../../test/fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { renderWithProviders } from '../../test/render'
import { GameSection } from './GameSection'

describe('GameSection', () => {
  it('shows the folder, Paks folder, source and build of a ready install', () => {
    installIpcMock()
    renderWithProviders(<GameSection />)

    expect(screen.getByText('/games/steamapps/common/TEKKEN 8')).toBeVisible()
    expect(screen.getByText('Found through Steam')).toBeVisible()
    expect(screen.getByText('Build 25221600')).toBeVisible()
    expect(screen.getByRole('button', { name: 'Open Paks folder' })).toBeVisible()
  })

  it('says so when the saved folder is missing and offers no Paks button', () => {
    const status = makeGameStatus({ location: { kind: 'missing', path: '/old/TEKKEN 8' } })
    installIpcMock(undefined, { gameStatus: status })
    renderWithProviders(<GameSection />, { gameStatus: status })

    expect(screen.getByText('/old/TEKKEN 8 no longer contains TEKKEN 8.')).toBeVisible()
    expect(screen.queryByRole('button', { name: 'Open Paks folder' })).not.toBeInTheDocument()
  })

  it('opens the chooser with the typed path form when the folder picker is cancelled', async () => {
    const user = userEvent.setup()
    const status = makeGameStatus({ candidates: [] })
    const ipc = installIpcMock(undefined, {
      gameStatus: status,
      handlers: { game_pick_location: () => ({ kind: 'cancelled' }) },
    })
    renderWithProviders(<GameSection />, { gameStatus: status })

    await user.click(screen.getByRole('button', { name: 'Change folder…' }))

    expect(await screen.findByLabelText('Path to the TEKKEN 8 folder')).toBeVisible()
    expect(ipc.callsOf('game_pick_location')).toHaveLength(1)
  })

  it('lists other installations and reveals the Paks folder', async () => {
    const user = userEvent.setup()
    const status = makeGameStatus({
      candidates: [makeCandidate({ id: 'x', root: '/other/TEKKEN 8' })],
    })
    const ipc = installIpcMock(undefined, { gameStatus: status })
    renderWithProviders(<GameSection />, { gameStatus: status })

    await user.click(screen.getByRole('button', { name: 'Other installations' }))
    const item = await screen.findByTestId('game-candidate')
    expect(within(item).getByText('/other/TEKKEN 8')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'Open Paks folder' }))
    await waitFor(() => {
      expect(ipc.callsOf('game_reveal')).toHaveLength(1)
    })
  })
})
