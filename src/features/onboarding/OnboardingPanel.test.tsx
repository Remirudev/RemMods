import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import type { GameStatus } from '../../lib/ipc/types'
import { makeCandidate, makeGameStatus } from '../../test/fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { renderWithProviders } from '../../test/render'
import type { IpcHandler } from '../../test/types'
import { OnboardingPanel } from './OnboardingPanel'

const NOT_SET: GameStatus = makeGameStatus({
  location: { kind: 'notSet' },
  candidates: [
    makeCandidate({ id: 'a', root: '/games/one/TEKKEN 8' }),
    makeCandidate({ id: 'b', root: '/games/two/TEKKEN 8', inUse: true }),
    makeCandidate({ id: 'c', root: '/games/three/TEKKEN 8', problem: 'notFound' }),
  ],
})

function renderOnboarding(handlers: Record<string, IpcHandler> = {}, status = NOT_SET) {
  const ipc = installIpcMock(undefined, { gameStatus: status, handlers })
  renderWithProviders(<OnboardingPanel status={status} />, { gameStatus: status })
  return ipc
}

function reject(error: object): IpcHandler {
  return () => {
    throw error
  }
}

describe('OnboardingPanel', () => {
  it('lists the candidates and marks the one in use', () => {
    renderOnboarding()

    const items = screen.getAllByTestId('game-candidate')
    expect(items).toHaveLength(3)
    expect(items[1]).toHaveTextContent('In use')
    const buttons = screen.getAllByRole('button', { name: 'Use this folder' })
    expect(buttons[0]).toBeEnabled()
    expect(buttons[1]).toBeDisabled()
    expect(buttons[2]).toBeDisabled()
    expect(items[2]).toHaveTextContent('No TEKKEN 8 game files were found there.')
  })

  it('sets the location from a candidate', async () => {
    const user = userEvent.setup()
    const ipc = renderOnboarding()

    await user.click(screen.getAllByRole('button', { name: 'Use this folder' })[0]!)

    await waitFor(() => {
      expect(ipc.callsOf('game_set_location')).toEqual([
        { command: 'game_set_location', payload: { choice: { kind: 'candidate', id: 'a' } } },
      ])
    })
  })

  it('shows an inline alert when the picked folder is invalid', async () => {
    const user = userEvent.setup()
    renderOnboarding({
      game_pick_location: () => ({ kind: 'invalid', reason: 'notFound', path: '/home/x/Games' }),
    })

    await user.click(screen.getByRole('button', { name: 'Choose folder…' }))

    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent('That is not a TEKKEN 8 folder')
    expect(alert).toHaveTextContent('/home/x/Games: No TEKKEN 8 game files were found there.')
  })

  it('passes the dialog title and opens the manual form when the pick is cancelled', async () => {
    const user = userEvent.setup()
    const ipc = renderOnboarding({ game_pick_location: () => ({ kind: 'cancelled' }) })

    await user.click(screen.getByRole('button', { name: 'Choose folder…' }))

    expect(
      await screen.findByText('No folder was chosen. You can also type the path.'),
    ).toBeVisible()
    expect(await screen.findByLabelText('Path to the TEKKEN 8 folder')).toBeVisible()
    expect(ipc.callsOf('game_pick_location')[0]?.payload).toEqual({
      title: 'Choose the TEKKEN 8 folder',
    })
  })

  it('shows a folder error from the typed path inline', async () => {
    const user = userEvent.setup()
    renderOnboarding({
      game_set_location: reject({ kind: 'invalidGameFolder', reason: 'ambiguous' }),
    })

    await user.click(screen.getByRole('button', { name: 'Enter the path instead' }))
    await user.type(await screen.findByLabelText('Path to the TEKKEN 8 folder'), '/games/tekken')
    await user.click(screen.getByRole('button', { name: 'Use this path' }))

    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent('/games/tekken: Several folders there differ only in case')
  })

  it('asks for the full path when the typed one is not absolute', async () => {
    const user = userEvent.setup()
    renderOnboarding({
      game_set_location: reject({ kind: 'invalidInput', field: 'path' }),
    })

    await user.click(screen.getByRole('button', { name: 'Enter the path instead' }))
    await user.type(await screen.findByLabelText('Path to the TEKKEN 8 folder'), 'tekken')
    await user.click(screen.getByRole('button', { name: 'Use this path' }))

    expect(await screen.findByText('Enter the full path to the folder.')).toBeVisible()
  })

  it('searches again with game_detect', async () => {
    const user = userEvent.setup()
    const ipc = renderOnboarding()

    await user.click(screen.getByRole('button', { name: 'Search again' }))

    await waitFor(() => {
      expect(ipc.callsOf('game_detect')).toHaveLength(1)
    })
  })

  it('translates the detection notes instead of showing raw codes', async () => {
    const user = userEvent.setup()
    const status = makeGameStatus({
      location: { kind: 'notSet' },
      notes: [
        { code: 'steamRoot', path: '/steam', line: null, detail: 'flatpak' },
        {
          code: 'vdfError',
          path: '/steam/libraryfolders.vdf',
          line: 14,
          detail: 'unterminatedString',
        },
        { code: 'manifestError', path: '/steam/appmanifest', line: null, detail: 'brandNewCode' },
      ],
    })
    renderOnboarding({}, status)

    await user.click(screen.getByRole('button', { name: 'What RemMods checked' }))

    const notes = screen.getByTestId('detection-notes')
    expect(notes).toHaveTextContent('Steam found at /steam · Steam (Flatpak)')
    expect(notes).toHaveTextContent(
      '/steam/libraryfolders.vdf could not be read · line 14 · a quoted text is not closed',
    )
    expect(notes).not.toHaveTextContent('brandNewCode')
    expect(notes).not.toHaveTextContent('unterminatedString')
    expect(notes).not.toHaveTextContent('vdfError')
  })
})
