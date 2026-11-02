import { Channel } from '@tauri-apps/api/core'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import type { AppUpdateChannel, AppUpdateCheck, AppUpdateProgress } from '../../lib/ipc/types'
import { TEST_UPDATE_CHANNEL } from '../../test/fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { renderWithProviders } from '../../test/render'
import type { IpcHandler } from '../../test/types'
import { UpdatesSection } from './UpdatesSection'

interface ProgressChannel {
  onmessage: (message: AppUpdateProgress) => void
}

const AVAILABLE: AppUpdateCheck = {
  kind: 'available',
  version: '0.2.0',
  date: '2026-11-02T10:00:00Z',
  notes: '<b>Fixes</b>\n- faster scans',
}

function configured(install: AppUpdateChannel['install']): AppUpdateChannel {
  return {
    install,
    selfUpdate: install === 'appImage' || install === 'nsis',
    configured: true,
    current: '0.1.0',
  }
}

function renderSection(channel: AppUpdateChannel, handlers: Record<string, IpcHandler> = {}) {
  const ipc = installIpcMock(undefined, {
    handlers: { update_channel: () => channel, update_check: () => AVAILABLE, ...handlers },
  })
  renderWithProviders(<UpdatesSection />)
  return ipc
}

async function checkNow(user: ReturnType<typeof userEvent.setup>) {
  await user.click(await screen.findByRole('button', { name: 'Check for updates' }))
}

async function confirmInstall(user: ReturnType<typeof userEvent.setup>) {
  await user.click(await screen.findByRole('button', { name: 'Install and restart' }))
  const dialog = await screen.findByRole('dialog', { name: 'Install RemMods 0.2.0?' })
  await user.click(within(dialog).getByRole('button', { name: 'Install and restart' }))
}

describe('UpdatesSection', () => {
  it('disables the check and says why when the build has no update channel', async () => {
    const ipc = renderSection(TEST_UPDATE_CHANNEL)

    expect(
      await screen.findByText('This build has no update channel (development or test build).'),
    ).toBeVisible()
    expect(screen.getByRole('button', { name: 'Check for updates' })).toBeDisabled()
    expect(screen.getByText('Unknown')).toBeVisible()
    expect(ipc.callsOf('update_check')).toHaveLength(0)
  })

  it('never checks on its own and says when the version is current', async () => {
    const user = userEvent.setup()
    const ipc = renderSection(configured('appImage'), {
      update_check: () => ({ kind: 'upToDate' }),
    })

    expect(await screen.findByText('AppImage')).toBeVisible()
    expect(ipc.callsOf('update_check')).toHaveLength(0)
    await checkNow(user)
    expect(await screen.findByText('RemMods 0.1.0 is up to date.')).toBeVisible()
    expect(ipc.callsOf('update_check')).toHaveLength(1)
  })

  it('installs an AppImage update after the confirmation and shows the progress', async () => {
    const user = userEvent.setup()
    let progress: ProgressChannel | null = null
    const ipc = renderSection(configured('appImage'), {
      update_install: (payload) => {
        progress = (payload as { progress: ProgressChannel }).progress
        progress.onmessage({ kind: 'downloading', downloaded: 50, total: 200 })
        return new Promise(() => undefined)
      },
    })

    await checkNow(user)
    expect(await screen.findByText('RemMods 0.2.0 is available.')).toBeVisible()
    expect(screen.getByText(/<b>Fixes<\/b>/)).toBeVisible()
    await user.click(screen.getByRole('button', { name: 'Install and restart' }))
    const dialog = await screen.findByRole('dialog', { name: 'Install RemMods 0.2.0?' })
    expect(within(dialog).getByText('RemMods will close to install the update.')).toBeVisible()
    expect(within(dialog).queryByText(/installer shows its progress/)).not.toBeInTheDocument()
    expect(ipc.callsOf('update_install')).toHaveLength(0)
    await user.click(within(dialog).getByRole('button', { name: 'Install and restart' }))

    expect(await screen.findByText('Downloading… 25 %')).toBeVisible()
    const [call] = ipc.callsOf('update_install')
    const payload = call?.payload as { version: string; progress: unknown }
    expect(payload.version).toBe('0.2.0')
    expect(payload.progress).toBeInstanceOf(Channel)
    expect(screen.getByRole('dialog', { name: 'Updating RemMods' })).toBeVisible()

    const channel = progress as ProgressChannel | null
    channel?.onmessage({ kind: 'installing' })
    expect(await screen.findByText('Installing…')).toBeVisible()
    channel?.onmessage({ kind: 'restarting' })
    expect(await screen.findByText('Restarting RemMods…')).toBeVisible()
  })

  it('shows the size instead of a percent when the server announces none', async () => {
    const user = userEvent.setup()
    renderSection(configured('nsis'), {
      update_install: (payload) => {
        const { progress } = payload as { progress: ProgressChannel }
        progress.onmessage({ kind: 'downloading', downloaded: 2048, total: null })
        return new Promise(() => undefined)
      },
    })

    await checkNow(user)
    await user.click(await screen.findByRole('button', { name: 'Install and restart' }))
    const dialog = await screen.findByRole('dialog', { name: 'Install RemMods 0.2.0?' })
    expect(
      within(dialog).getByText('The installer shows its progress and starts RemMods again.'),
    ).toBeVisible()
    await user.click(within(dialog).getByRole('button', { name: 'Install and restart' }))
    expect(await screen.findByText('Downloading… 2 KB')).toBeVisible()
  })

  it('gives a package install its notice and no install button', async () => {
    const user = userEvent.setup()
    const ipc = renderSection(configured('deb'))

    await checkNow(user)
    expect(
      await screen.findByText(
        'Installed from a package: download the new .deb from the releases page and install it with your package manager.',
      ),
    ).toBeVisible()
    expect(screen.getByText('https://github.com/Remirudev/RemMods/releases')).toBeVisible()
    expect(screen.queryByRole('button', { name: 'Install and restart' })).not.toBeInTheDocument()
    expect(ipc.callsOf('update_install')).toHaveLength(0)
  })

  it('shows the AUR helper notice before any check, even without an update channel', async () => {
    renderSection({ ...TEST_UPDATE_CHANNEL, install: 'aur' })

    expect(await screen.findByText('AUR package (remmods-bin)')).toBeVisible()
    expect(
      screen.getByText(
        'Installed from the AUR: update with your AUR helper, for example paru -Syu.',
      ),
    ).toBeVisible()
    expect(
      screen.getByText('This build has no update channel (development or test build).'),
    ).toBeVisible()
  })

  it('gives an AUR install the AUR helper notice', async () => {
    const user = userEvent.setup()
    renderSection(configured('aur'))

    await checkNow(user)
    expect(await screen.findByText('RemMods 0.2.0 is available.')).toBeVisible()
    expect(
      await screen.findByText(
        'Installed from the AUR: update with your AUR helper, for example paru -Syu.',
      ),
    ).toBeVisible()
    expect(screen.queryByRole('button', { name: 'Install and restart' })).not.toBeInTheDocument()
  })

  it('closes the progress and shows the translated error when the install fails', async () => {
    const user = userEvent.setup()
    renderSection(configured('appImage'), {
      update_install: () => {
        throw { kind: 'updateFailed', stage: 'verify' }
      },
    })

    await checkNow(user)
    await confirmInstall(user)

    expect(
      await screen.findByText(
        'The downloaded update failed the signature check, so it was not installed.',
      ),
    ).toBeVisible()
    await waitFor(() =>
      expect(screen.queryByRole('dialog', { name: 'Updating RemMods' })).not.toBeInTheDocument(),
    )
    expect(screen.getByRole('button', { name: 'Check for updates' })).toBeEnabled()
  })

  it('shows a failed check inside the section', async () => {
    const user = userEvent.setup()
    renderSection(configured('appImage'), {
      update_check: () => {
        throw { kind: 'updateFailed', stage: 'check' }
      },
    })

    await checkNow(user)
    expect(await screen.findByText(/could not check for updates/)).toBeVisible()
  })
})
