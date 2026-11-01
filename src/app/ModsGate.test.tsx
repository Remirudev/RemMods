import { screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { makeGameStatus } from '../test/fixtures'
import { installIpcMock } from '../test/ipc-mock'
import { mockElementLayout } from '../test/layout'
import { renderWithProviders } from '../test/render'
import { ModsGate } from './ModsGate'

describe('ModsGate', () => {
  let restoreLayout: () => void

  beforeEach(() => {
    restoreLayout = mockElementLayout()
  })

  afterEach(() => {
    restoreLayout()
  })

  it('labels its loader while the game status is pending', () => {
    installIpcMock(undefined, { handlers: { game_status: () => new Promise(() => undefined) } })
    renderWithProviders(<ModsGate />, { gameStatus: null })

    expect(screen.getByLabelText('Looking for TEKKEN 8…')).toBeInTheDocument()
    expect(screen.getByText('Looking for TEKKEN 8…')).toBeVisible()
  })

  it('shows an alert with a retry when the status cannot be read', async () => {
    installIpcMock(undefined, {
      handlers: {
        game_status: () => {
          throw { kind: 'internal' }
        },
      },
    })
    renderWithProviders(<ModsGate />, { gameStatus: null })

    expect(await screen.findByText('Unexpected error')).toBeVisible()
    expect(screen.getByRole('button', { name: 'Try again' })).toBeVisible()
  })

  it('shows the onboarding when no game folder is set', () => {
    installIpcMock()
    renderWithProviders(<ModsGate />, {
      gameStatus: makeGameStatus({ location: { kind: 'notSet' } }),
    })

    expect(screen.getByTestId('onboarding')).toBeInTheDocument()
    expect(screen.getByText('Where is TEKKEN 8?')).toBeVisible()
  })

  it('shows the onboarding with a warning when the saved folder is gone', () => {
    installIpcMock()
    renderWithProviders(<ModsGate />, {
      gameStatus: makeGameStatus({ location: { kind: 'missing', path: '/old/TEKKEN 8' } }),
    })

    expect(screen.getByTestId('onboarding')).toBeInTheDocument()
    expect(screen.getByText('The saved game folder is gone')).toBeVisible()
    expect(screen.getByText(/\/old\/TEKKEN 8 no longer contains TEKKEN 8/)).toBeVisible()
  })

  it('shows the mods panel once a game folder is set', async () => {
    installIpcMock()
    renderWithProviders(<ModsGate />)

    expect(await screen.findByRole('heading', { name: 'Mods' })).toBeVisible()
    expect(await screen.findAllByTestId('mods-row')).toHaveLength(3)
    expect(screen.queryByTestId('onboarding')).not.toBeInTheDocument()
  })
})
