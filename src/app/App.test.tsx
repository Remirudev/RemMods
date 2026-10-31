import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Settings } from '../lib/ipc/types'
import { TEST_SETTINGS } from '../test/fixtures'
import { installIpcMock } from '../test/ipc-mock'
import { renderWithProviders } from '../test/render'
import { App } from './App'
import { useActivePanelStore } from './active-panel-store'

const appWindow = vi.hoisted(() => ({
  minimize: vi.fn(() => Promise.resolve()),
  toggleMaximize: vi.fn(() => Promise.resolve()),
  close: vi.fn(() => Promise.resolve()),
  isMaximized: vi.fn(() => Promise.resolve(false)),
  onResized: vi.fn(() => Promise.resolve(() => undefined)),
  startResizeDragging: vi.fn(() => Promise.resolve()),
}))

vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => appWindow }))

describe('App shell', () => {
  beforeEach(() => {
    useActivePanelStore.setState({ activePanel: 'mods', visited: ['mods'] })
  })

  it('switches panels from the rail and keeps the visited one mounted', async () => {
    installIpcMock()
    const user = userEvent.setup()
    renderWithProviders(<App />)

    expect(await screen.findByRole('heading', { name: 'Mods' })).toBeVisible()
    expect(screen.getByTestId('rail')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'Profiles' }))

    expect(await screen.findByRole('heading', { name: 'Profiles' })).toBeVisible()
    expect(screen.getByRole('heading', { name: 'Mods', hidden: true })).not.toBeVisible()
    expect(localStorage.getItem('rmods.activePanel')).toBe('profiles')
  })

  it('renders the custom title bar only when the title bar setting is custom', async () => {
    installIpcMock()
    renderWithProviders(<App />)

    expect(await screen.findByTestId('title-bar')).toBeInTheDocument()
  })

  it('leaves the frame to the system when the title bar setting is system', async () => {
    const settings: Settings = {
      ...TEST_SETTINGS,
      appearance: { ...TEST_SETTINGS.appearance, titleBar: 'system' },
    }
    installIpcMock(settings)
    renderWithProviders(<App />, { settings })

    expect(await screen.findByRole('heading', { name: 'Mods' })).toBeVisible()
    expect(screen.queryByTestId('title-bar')).not.toBeInTheDocument()
  })
})
