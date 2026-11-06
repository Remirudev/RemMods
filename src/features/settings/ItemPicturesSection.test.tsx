import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import { installIpcMock } from '../../test/ipc-mock'
import { renderWithProviders } from '../../test/render'
import { ItemPicturesSection } from './ItemPicturesSection'

const STORED = { pictures: 1234, bytes: 45_000_000, names: 23_462, failed: 0 }

describe('ItemPicturesSection', () => {
  it('says what is stored', async () => {
    installIpcMock(undefined, { handlers: { assets_status: () => STORED } })
    renderWithProviders(<ItemPicturesSection />)

    expect(await screen.findByText(/1,234 pictures/)).toBeVisible()
    expect(screen.getByText(/23,462 names/)).toBeVisible()
    expect(screen.queryByText(/could not be read/)).toBeNull()
  })

  it('says so while nothing is stored, and counts pictures that failed', async () => {
    installIpcMock(undefined, {
      handlers: {
        assets_status: () => ({ pictures: 0, bytes: 0, names: 0, failed: 2 }),
      },
    })
    renderWithProviders(<ItemPicturesSection />)

    expect(await screen.findByText(/Nothing stored yet/)).toBeVisible()
    expect(screen.getByText('2 pictures could not be read.')).toBeVisible()
  })

  it('checks on request and shows the new numbers', async () => {
    const ipc = installIpcMock(undefined, {
      handlers: {
        assets_status: () => ({ pictures: 0, bytes: 0, names: 0, failed: 0 }),
        assets_refresh: () => STORED,
      },
    })
    renderWithProviders(<ItemPicturesSection />)
    await screen.findByText(/Nothing stored yet/)

    await userEvent.click(screen.getByRole('button', { name: 'Check now' }))

    expect(await screen.findByText(/1,234 pictures/)).toBeVisible()
    await waitFor(() => expect(ipc.callsOf('assets_refresh')).toHaveLength(1))
  })

  it('turns the big picture on hover off through the settings', async () => {
    const ipc = installIpcMock()
    renderWithProviders(<ItemPicturesSection />)
    const hover = screen.getByRole('switch', { name: /Big picture on hover/ })
    expect(hover).toBeChecked()

    await userEvent.click(hover)

    await waitFor(() => expect(ipc.callsOf('settings_update')).toHaveLength(1))
    expect(ipc.callsOf('settings_update')[0]?.payload).toEqual({
      update: { appearance: { hoverPreview: false } },
    })
  })

  it('hides pictures through the settings', async () => {
    const ipc = installIpcMock()
    renderWithProviders(<ItemPicturesSection />)

    await userEvent.click(screen.getByRole('switch', { name: /Hide pictures/ }))

    await waitFor(() => expect(ipc.callsOf('settings_update')).toHaveLength(1))
    expect(ipc.callsOf('settings_update')[0]?.payload).toEqual({
      update: { appearance: { hidePictures: true } },
    })
  })
})
