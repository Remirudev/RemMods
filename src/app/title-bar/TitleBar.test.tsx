import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { renderWithProviders } from '../../test/render'
import { TitleBar } from './TitleBar'

const appWindow = vi.hoisted(() => ({
  minimize: vi.fn(() => Promise.resolve()),
  toggleMaximize: vi.fn(() => Promise.resolve()),
  close: vi.fn(() => Promise.resolve()),
  isMaximized: vi.fn(() => Promise.resolve(false)),
  onResized: vi.fn(() => Promise.resolve(() => undefined)),
  startResizeDragging: vi.fn(() => Promise.resolve()),
}))

vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => appWindow }))

describe('TitleBar', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    appWindow.isMaximized.mockImplementation(() => Promise.resolve(false))
  })

  it('wires minimise, maximise and close to the window API', async () => {
    const user = userEvent.setup()
    renderWithProviders(<TitleBar />)

    expect(screen.getByTestId('title-bar')).toHaveAttribute('data-tauri-drag-region')
    await user.click(screen.getByRole('button', { name: 'Minimize' }))
    await user.click(screen.getByRole('button', { name: 'Maximize' }))
    await user.click(screen.getByRole('button', { name: 'Close' }))

    expect(appWindow.minimize).toHaveBeenCalledOnce()
    expect(appWindow.toggleMaximize).toHaveBeenCalledOnce()
    expect(appWindow.close).toHaveBeenCalledOnce()
  })

  it('offers Restore while the window is maximised', async () => {
    appWindow.isMaximized.mockImplementation(() => Promise.resolve(true))
    renderWithProviders(<TitleBar />)

    expect(await screen.findByRole('button', { name: 'Restore' })).toBeInTheDocument()
    await waitFor(() => expect(appWindow.onResized).toHaveBeenCalled())
  })
})
