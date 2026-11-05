import { Notifications } from '@mantine/notifications'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import type { SkinsView } from '../../lib/ipc/types'
import { makeSkinSet, TEST_SKINS_VIEW } from '../../test/fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { renderWithProviders } from '../../test/render'
import { SkinSetsSection } from './SkinSetsSection'

const TWO_SETS: SkinsView = {
  ...TEST_SKINS_VIEW,
  sets: [makeSkinSet('First'), makeSkinSet('Second')],
  active: 'first',
}

function renderSection(view: SkinsView, handlers = {}) {
  const ipc = installIpcMock(undefined, { handlers: { skins_view: () => view, ...handlers } })
  renderWithProviders(
    <>
      <Notifications />
      <SkinSetsSection />
    </>,
  )
  return ipc
}

describe('SkinSetsSection', () => {
  it('says so when the game’s saves are not there', async () => {
    renderSection({ ...TEST_SKINS_VIEW, available: false, account: null })
    expect(await screen.findByText(/No TEKKEN 8 skin saves found/)).toBeVisible()
    expect(screen.queryByRole('button', { name: /Save current skins/ })).not.toBeInTheDocument()
  })

  it('offers to save the first set', async () => {
    renderSection(TEST_SKINS_VIEW)
    expect(await screen.findByText(/more than the game has slots/)).toBeVisible()
    expect(screen.getByRole('button', { name: 'Save current skins as a set' })).toBeVisible()
    expect(screen.getByRole('button', { name: 'Save my skins' })).toBeVisible()
  })

  it('hides the automatic copies when the game’s saves are not there', async () => {
    renderSection({ ...TEST_SKINS_VIEW, available: false, account: null })
    await screen.findByText(/No TEKKEN 8 skin saves found/)
    expect(screen.queryByRole('button', { name: 'Automatic copies' })).not.toBeInTheDocument()
  })

  it('marks the set in the game and offers a switch to the others', async () => {
    renderSection(TWO_SETS)
    const rows = await screen.findAllByTestId('skin-set-row')
    expect(within(rows[0] as HTMLElement).getByText('In the game')).toBeVisible()
    expect(within(rows[0] as HTMLElement).queryByRole('button', { name: /Switch/ })).toBeNull()
    expect(screen.getByRole('button', { name: 'Switch to Second' })).toBeVisible()
  })

  it('switches and says what happened', async () => {
    const ipc = renderSection(TWO_SETS, {
      skins_switch: () => ({ ...TWO_SETS, active: 'second' }),
    })

    await userEvent.click(await screen.findByRole('button', { name: 'Switch to Second' }))

    await waitFor(() => expect(ipc.callsOf('skins_switch')).toHaveLength(1))
    expect(ipc.callsOf('skins_switch')[0]?.payload).toEqual({ id: 'second' })
    expect(await screen.findByText(/Skins switched to Second/)).toBeVisible()
    expect(await screen.findByRole('button', { name: 'Switch to First' })).toBeVisible()
  })

  it('tells the user when the game is running', async () => {
    renderSection(TWO_SETS, {
      skins_switch: () => {
        throw { kind: 'gameRunning' }
      },
    })

    await userEvent.click(await screen.findByRole('button', { name: 'Switch to Second' }))

    expect(await screen.findByText('TEKKEN 8 is running')).toBeVisible()
  })

  it('holds everything back until an interrupted switch is finished', async () => {
    const ipc = renderSection(
      { ...TWO_SETS, interrupted: 'second' },
      { skins_switch: () => ({ ...TWO_SETS, active: 'second' }) },
    )

    expect(await screen.findByText(/A switch to Second did not finish/)).toBeVisible()
    expect(screen.getByRole('button', { name: 'Switch to Second' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Save current skins as a set' })).toBeDisabled()

    await userEvent.click(screen.getByRole('button', { name: 'Finish the switch' }))

    await waitFor(() => expect(ipc.callsOf('skins_switch')).toHaveLength(1))
  })

  it('only shows a list from a newer RemMods', async () => {
    renderSection({ ...TWO_SETS, readOnly: true })
    expect(await screen.findByText(/comes from a newer RemMods/)).toBeVisible()
    expect(screen.getByRole('button', { name: 'Switch to Second' })).toBeDisabled()
  })
})
