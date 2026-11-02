import { act, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { AppUpdateChannel, Settings } from '../lib/ipc/types'
import { useInstallQueue } from '../shared/install-queue'
import { useUpdateRun } from '../shared/update-run-store'
import { makeGraphicsState, TEST_SETTINGS, WINDOWS_PLATFORM_INFO } from '../test/fixtures'
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

  it('shows the graphics step banner above the panels after a crashed start', async () => {
    const stepped = makeGraphicsState({ step: { from: 'auto', to: 'software' } })
    installIpcMock(undefined, { handlers: { graphics_get: () => stepped } })
    renderWithProviders(<App />)

    expect(await screen.findByTestId('graphics-step-banner')).toBeVisible()
    expect(screen.getByRole('button', { name: 'Switch back' })).toBeVisible()
  })

  it('shows the graphics step banner only once on the Settings panel', async () => {
    useActivePanelStore.setState({ activePanel: 'settings', visited: ['settings'] })
    const stepped = makeGraphicsState({ step: { from: 'auto', to: 'software' } })
    installIpcMock(undefined, { handlers: { graphics_get: () => stepped } })
    renderWithProviders(<App />)

    expect(await screen.findByTestId('graphics-step-banner')).toBeVisible()
    expect(screen.getAllByTestId('graphics-step-banner')).toHaveLength(1)
    expect(screen.getAllByRole('button', { name: 'Switch back' })).toHaveLength(1)
  })

  it('never shows the graphics banner on Windows', async () => {
    const ipc = installIpcMock()
    renderWithProviders(<App />, { platformInfo: WINDOWS_PLATFORM_INFO })

    expect(await screen.findByRole('heading', { name: 'Mods' })).toBeVisible()
    expect(screen.queryByTestId('graphics-step-banner')).not.toBeInTheDocument()
    expect(ipc.callsOf('graphics_get')).toHaveLength(0)
  })
})

const SELF_UPDATE: AppUpdateChannel = {
  install: 'appImage',
  selfUpdate: true,
  configured: true,
  current: '0.1.0',
}
const PALETTE = 'Type a command…'

async function startUpdate(user: ReturnType<typeof userEvent.setup>) {
  await user.click(await screen.findByRole('button', { name: 'Check for updates' }))
  await user.click(await screen.findByRole('button', { name: 'Install and restart' }))
  const confirm = await screen.findByRole('dialog', { name: 'Install RemMods 0.2.0?' })
  await user.click(within(confirm).getByRole('button', { name: 'Install and restart' }))
  expect(await screen.findByRole('dialog', { name: 'Updating RemMods' })).toBeVisible()
}

describe('App shell while a self-update runs', () => {
  beforeEach(() => {
    useActivePanelStore.setState({ activePanel: 'settings', visited: ['settings'] })
    useInstallQueue.getState().clear()
    installIpcMock(undefined, {
      handlers: {
        update_channel: () => SELF_UPDATE,
        update_check: () => ({ kind: 'available', version: '0.2.0', date: null, notes: null }),
        update_install: () => new Promise(() => undefined),
      },
    })
  })

  afterEach(() => useUpdateRun.setState({ updating: false }))

  it('opens no palette on Ctrl+K and keeps the progress dialog on the Settings panel', async () => {
    const user = userEvent.setup()
    renderWithProviders(<App />)

    await startUpdate(user)
    expect(useUpdateRun.getState().updating).toBe(true)
    await user.keyboard('{Control>}k{/Control}')
    expect(screen.queryByPlaceholderText(PALETTE)).not.toBeInTheDocument()

    useActivePanelStore.getState().setActivePanel('mods')
    expect(useActivePanelStore.getState().activePanel).toBe('settings')
    expect(screen.getByRole('dialog', { name: 'Updating RemMods' })).toBeVisible()
  })

  it('closes an open palette when the update starts and queues no install while it runs', async () => {
    const user = userEvent.setup()
    renderWithProviders(<App />)

    expect(await screen.findByRole('button', { name: 'Check for updates' })).toBeVisible()
    await user.keyboard('{Control>}k{/Control}')
    expect(await screen.findByPlaceholderText(PALETTE)).toBeVisible()
    act(() => useUpdateRun.getState().setUpdating(true))
    await waitFor(() => expect(screen.queryByPlaceholderText(PALETTE)).not.toBeInTheDocument())

    useInstallQueue.getState().push([{ kind: 'archive', row: 'row-1', name: 'one.zip' }])
    expect(useInstallQueue.getState().current).toBeNull()
  })
})
